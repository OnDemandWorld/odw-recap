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
    /// Open storage, loading the persisted salt or creating it on first run.
    pub fn new(data_dir: PathBuf, passphrase: &str) -> Result<Self> {
        std::fs::create_dir_all(&data_dir)?;
        let salt = Self::load_or_create_salt(&data_dir)?;
        Self::with_salt(data_dir, passphrase, salt)
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

    /// Load the salt from disk, or generate and persist it on first run.
    fn load_or_create_salt(data_dir: &Path) -> Result<[u8; 32]> {
        let salt_path = data_dir.join(SALT_FILE);
        if salt_path.exists() {
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
            return Ok(salt);
        }

        let salt = EncryptionManager::generate_salt();
        Self::write_private_file(&salt_path, &salt)?;
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

    /// Change the passphrase.
    ///
    /// Currently unimplemented on purpose: swapping the key without
    /// re-encrypting existing data would silently make encrypted API keys and
    /// files unreadable. A correct implementation must re-encrypt every
    /// encrypted artifact (api_keys table, encrypted files) atomically.
    #[allow(dead_code)]
    pub fn change_passphrase(&self, _new_passphrase: &str) -> Result<()> {
        Err(RecapError::Encryption(
            "Passphrase change is not implemented yet; it would need to re-encrypt all stored data"
                .to_string(),
        ))
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
