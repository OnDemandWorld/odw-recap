pub mod encryption_manager;
pub mod file_store;
pub mod sqlite_manager;
pub mod types;

#[cfg(test)]
mod tests;

use crate::error::{RecapError, Result};
use crate::storage::encryption_manager::EncryptionManager;
use crate::storage::file_store::FileStore;
use crate::storage::sqlite_manager::SqliteManager;
use crate::storage::types::*;
use std::path::{Path, PathBuf};
use uuid::Uuid;

/// File (inside the data directory) that stores the KDF salt. The salt is
/// generated once per installation and must never change, otherwise data
/// encrypted with the derived key becomes unrecoverable.
const SALT_FILE: &str = "recap.salt";
const SALT_LEN: usize = 32;

/// Configuration key holding the encrypted vault sentinel. Decrypting it with
/// the current master key verifies the passphrase; a decryption failure means
/// the wrong passphrase was supplied.
const VAULT_CHECK_KEY: &str = "vault_check";
const VAULT_SENTINEL: &[u8] = b"recap-vault-sentinel-v1";

/// Returns true once a vault has been created in `data_dir` (salt exists).
pub fn vault_initialized(data_dir: &Path) -> bool {
    data_dir.join(SALT_FILE).exists()
}

pub struct StorageManager {
    db: SqliteManager,
    file_store: FileStore,
    // Accessed through the reserved accessors below.
    #[allow(dead_code)]
    encryption: EncryptionManager,
    #[allow(dead_code)]
    data_dir: PathBuf,
}

impl StorageManager {
    /// Create a brand-new vault. Fails if one already exists in `data_dir`.
    pub fn initialize(data_dir: PathBuf, passphrase: &str) -> Result<Self> {
        if vault_initialized(&data_dir) {
            return Err(RecapError::Vault(
                "Vault is already initialized".to_string(),
            ));
        }

        std::fs::create_dir_all(&data_dir)?;
        let salt = EncryptionManager::generate_salt();
        Self::write_private_file(&data_dir.join(SALT_FILE), &salt)?;

        let manager = Self::with_salt(data_dir, passphrase, salt)?;
        manager.write_sentinel()?;
        Ok(manager)
    }

    /// Open an existing vault. Fails if the vault was never initialized or
    /// the passphrase does not match.
    pub fn open(data_dir: PathBuf, passphrase: &str) -> Result<Self> {
        let salt = Self::load_salt(&data_dir)?;
        let manager = Self::with_salt(data_dir, passphrase, salt)?;
        manager.verify_or_adopt_sentinel()?;
        Ok(manager)
    }

    /// Convenience constructor (tests, automation, future CLI tooling):
    /// initialize the vault on first use, otherwise open it.
    #[allow(dead_code)]
    pub fn open_or_initialize(data_dir: PathBuf, passphrase: &str) -> Result<Self> {
        if vault_initialized(&data_dir) {
            Self::open(data_dir, passphrase)
        } else {
            Self::initialize(data_dir, passphrase)
        }
    }

    pub fn with_salt(data_dir: PathBuf, passphrase: &str, salt: [u8; 32]) -> Result<Self> {
        let encryption = EncryptionManager::with_salt(salt);
        encryption.unlock(passphrase)?;

        let file_store = FileStore::new(data_dir.clone(), encryption.clone());
        file_store.init()?;

        let db_path = data_dir.join("recap.db");
        let db = SqliteManager::new(&db_path)?;

        Ok(Self {
            db,
            file_store,
            encryption,
            data_dir,
        })
    }

    /// Load the persisted salt; an error means the vault is not initialized.
    fn load_salt(data_dir: &Path) -> Result<[u8; 32]> {
        let salt_path = data_dir.join(SALT_FILE);
        if !salt_path.exists() {
            return Err(RecapError::Vault(
                "Vault is not initialized on this device".to_string(),
            ));
        }
        let bytes = std::fs::read(&salt_path)?;
        if bytes.len() != SALT_LEN {
            return Err(RecapError::Encryption(format!(
                "Corrupt salt file (expected {} bytes, found {}): {}",
                SALT_LEN,
                bytes.len(),
                salt_path.display()
            )));
        }
        let mut salt = [0u8; SALT_LEN];
        salt.copy_from_slice(&bytes);
        Ok(salt)
    }

    /// Write a file with owner-only permissions on Unix.
    fn write_private_file(path: &Path, data: &[u8]) -> Result<()> {
        #[cfg(unix)]
        {
            use std::io::Write;
            use std::os::unix::fs::OpenOptionsExt;
            let mut file = std::fs::OpenOptions::new()
                .create_new(true)
                .write(true)
                .mode(0o600)
                .open(path)?;
            file.write_all(data)?;
        }
        #[cfg(not(unix))]
        {
            std::fs::write(path, data)?;
        }
        Ok(())
    }

    /// Encrypt the sentinel with the current master key and store it.
    fn write_sentinel(&self) -> Result<()> {
        use base64::{Engine as _, engine::general_purpose::STANDARD};
        let encrypted = self.encryption.encrypt(VAULT_SENTINEL)?;
        self.db
            .set_config_value(VAULT_CHECK_KEY, &STANDARD.encode(&encrypted))
    }

    /// Verify the passphrase against the stored sentinel. If no sentinel
    /// exists yet (database created before the vault flow was introduced),
    /// adopt the current passphrase by writing one.
    fn verify_or_adopt_sentinel(&self) -> Result<()> {
        use base64::{Engine as _, engine::general_purpose::STANDARD};

        match self.db.get_config_value(VAULT_CHECK_KEY)? {
            Some(b64) => {
                let encrypted = STANDARD.decode(&b64).map_err(|e| {
                    RecapError::Encryption(format!("Corrupt vault sentinel: {}", e))
                })?;
                let decrypted = self.encryption.decrypt(&encrypted).map_err(|_| {
                    RecapError::Vault("Wrong passphrase".to_string())
                })?;
                if decrypted != VAULT_SENTINEL {
                    return Err(RecapError::Vault("Wrong passphrase".to_string()));
                }
                Ok(())
            }
            None => self.write_sentinel(),
        }
    }

    /// Change the passphrase safely: every encrypted artifact (all API keys
    /// and the vault sentinel) is re-encrypted with the new key inside a
    /// single database transaction. The in-memory master key is swapped only
    /// after the transaction commits, so a failure leaves all data readable
    /// with the old passphrase.
    pub fn change_passphrase(&self, new_passphrase: &str) -> Result<()> {
        use base64::{Engine as _, engine::general_purpose::STANDARD};

        if new_passphrase.len() < 8 {
            return Err(RecapError::Vault(
                "New passphrase must be at least 8 characters".to_string(),
            ));
        }

        let new_key = self.encryption.derive_key(new_passphrase)?;

        // Re-encrypt every stored API key: old key decrypts, new key encrypts.
        let mut updates = Vec::new();
        for (provider, encrypted_b64) in self.db.list_api_keys_encrypted()? {
            let encrypted = STANDARD.decode(&encrypted_b64).map_err(|e| {
                RecapError::Encryption(format!("Corrupt API key entry: {}", e))
            })?;
            let plaintext = self.encryption.decrypt(&encrypted)?;
            let reencrypted = EncryptionManager::encrypt_with_key(&new_key, &plaintext)?;
            updates.push((provider, STANDARD.encode(&reencrypted)));
        }

        // Sentinel under the new key, committed atomically with the keys.
        let new_sentinel = STANDARD.encode(
            &EncryptionManager::encrypt_with_key(&new_key, VAULT_SENTINEL)?,
        );
        self.db.apply_reencryption(&updates, &new_sentinel)?;

        // Everything is persisted under the new key; swap the master key.
        self.encryption.replace_key(new_key)
    }

    #[allow(dead_code)]
    pub fn get_data_dir(&self) -> &Path {
        &self.data_dir
    }

    #[allow(dead_code)]
    pub fn get_encryption(&self) -> &EncryptionManager {
        &self.encryption
    }

    #[allow(dead_code)]
    pub fn get_db(&self) -> &SqliteManager {
        &self.db
    }

    pub fn get_file_store(&self) -> &FileStore {
        &self.file_store
    }

    // Meeting operations
    pub fn create_meeting(&self, meeting: &Meeting) -> Result<()> {
        self.db.create_meeting(meeting)
    }

    pub fn get_meeting(&self, id: Uuid) -> Result<Option<Meeting>> {
        self.db.get_meeting(id)
    }

    pub fn update_meeting(&self, meeting: &Meeting) -> Result<()> {
        self.db.update_meeting(meeting)
    }

    pub fn list_meetings(&self, limit: i64, offset: i64) -> Result<Vec<Meeting>> {
        self.db.list_meetings(limit, offset)
    }

    pub fn search_transcripts(&self, query: &str, limit: i64) -> Result<Vec<(Uuid, String)>> {
        self.db.search_transcripts(query, limit)
    }

    // Transcript operations
    pub fn insert_transcript_segment(&self, segment: &TranscriptSegment) -> Result<()> {
        self.db.insert_transcript_segment(segment)
    }

    pub fn get_transcript_segments(&self, meeting_id: Uuid) -> Result<Vec<TranscriptSegment>> {
        self.db.get_transcript_segments(meeting_id)
    }

    // Summary operations
    pub fn save_summary(&self, summary: &Summary) -> Result<()> {
        self.db.save_summary(summary)
    }

    pub fn get_summary(&self, meeting_id: Uuid) -> Result<Option<Summary>> {
        self.db.get_summary(meeting_id)
    }

    // Action item operations
    pub fn save_action_item(&self, item: &ActionItem) -> Result<()> {
        self.db.save_action_item(item)
    }

    pub fn list_action_items(&self, meeting_id: Uuid) -> Result<Vec<ActionItem>> {
        self.db.list_action_items(meeting_id)
    }

    pub fn update_action_item_status(&self, id: Uuid, status: &str) -> Result<()> {
        self.db.update_action_item_status(id, status)
    }

    pub fn delete_action_item(&self, id: Uuid) -> Result<()> {
        self.db.delete_action_item(id)
    }

    // Decision operations
    pub fn save_decision(&self, decision: &Decision) -> Result<()> {
        self.db.save_decision(decision)
    }

    pub fn list_decisions(&self, meeting_id: Uuid) -> Result<Vec<Decision>> {
        self.db.list_decisions(meeting_id)
    }

    pub fn delete_decision(&self, id: Uuid) -> Result<()> {
        self.db.delete_decision(id)
    }

    // API key operations
    pub fn save_api_key(&self, provider: &str, api_key: &str) -> Result<()> {
        use base64::{Engine as _, engine::general_purpose::STANDARD};
        let encrypted = self.encryption.encrypt(api_key.as_bytes())?;
        let encrypted_b64 = STANDARD.encode(&encrypted);
        self.db.save_api_key(provider, &encrypted_b64)
    }

    pub fn get_api_key(&self, provider: &str) -> Result<Option<String>> {
        use base64::{Engine as _, engine::general_purpose::STANDARD};
        let encrypted_b64 = self.db.get_api_key(provider)?;
        match encrypted_b64 {
            Some(b64) => {
                let encrypted = STANDARD.decode(&b64).map_err(|e| {
                    RecapError::Encryption(format!("Failed to decode API key: {}", e))
                })?;
                let decrypted = self.encryption.decrypt(&encrypted)?;
                let key = String::from_utf8(decrypted).map_err(|e| {
                    RecapError::Encryption(format!("Invalid API key encoding: {}", e))
                })?;
                Ok(Some(key))
            }
            None => Ok(None),
        }
    }

    // Prompt template operations
    pub fn save_prompt_template(&self, name: &str, content: &str, meeting_type: Option<&str>, is_custom: bool) -> Result<()> {
        self.db.save_prompt_template(name, content, meeting_type, is_custom)
    }

    pub fn get_prompt_template(&self, name: &str) -> Result<Option<(String, Option<String>, bool)>> {
        self.db.get_prompt_template(name)
    }

    pub fn list_prompt_templates(&self) -> Result<Vec<(String, String, Option<String>, bool)>> {
        self.db.list_prompt_templates()
    }

    // File operations
    #[allow(dead_code)]
    pub fn write_encrypted_file(&self, relative_path: &Path, data: &[u8]) -> Result<()> {
        self.file_store.write_encrypted_file(relative_path, data)
    }

    #[allow(dead_code)]
    pub fn read_encrypted_file(&self, relative_path: &Path) -> Result<Vec<u8>> {
        self.file_store.read_encrypted_file(relative_path)
    }

    #[allow(dead_code)]
    pub fn write_plain_file(&self, relative_path: &Path, data: &[u8]) -> Result<()> {
        self.file_store.write_plain_file(relative_path, data)
    }

    #[allow(dead_code)]
    pub fn read_plain_file(&self, relative_path: &Path) -> Result<Vec<u8>> {
        self.file_store.read_plain_file(relative_path)
    }
}
