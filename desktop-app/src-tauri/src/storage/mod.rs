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

pub struct StorageManager {
    db: SqliteManager,
    file_store: FileStore,
    encryption: EncryptionManager,
    data_dir: PathBuf,
}

// Filename holding the persisted random Argon2 salt next to the database.
const SALT_FILE: &str = "salt.bin";

/// Load (or create) the per-installation random salt.
///
/// New installations get a random 32-byte salt persisted to
/// `<data_dir>/salt.bin` so the same passphrase keeps deriving the same key
/// across restarts. Existing installations created before salts were
/// persisted used the historical all-zero salt; they keep working (with a
/// warning) instead of silently losing access to already-encrypted API keys.
fn load_or_create_salt(data_dir: &Path) -> Result<[u8; 32]> {
    let salt_path = data_dir.join(SALT_FILE);
    if salt_path.exists() {
        let bytes = std::fs::read(&salt_path)?;
        if bytes.len() == 32 {
            let mut salt = [0u8; 32];
            salt.copy_from_slice(&bytes);
            return Ok(salt);
        }
        // Audit #49: a corrupt salt file previously made StorageManager::new
        // fail and main() .expect() panic before any window existed (invisible
        // on Windows release builds). Quarantine it and create a fresh salt so
        // the app can launch; previously saved API keys must be re-entered.
        eprintln!(
            "WARNING: corrupted salt file at {} (expected 32 bytes, found {}); \
             quarantining to salt.bin.corrupt and creating a fresh salt. \
             Previously saved API keys can no longer be decrypted and must be re-saved.",
            salt_path.display(),
            bytes.len()
        );
        let _ = std::fs::rename(&salt_path, data_dir.join("salt.bin.corrupt"));
        let salt = EncryptionManager::generate_salt();
        write_salt_atomic(&salt_path, &salt)?;
        return Ok(salt);
    }

    // No salt file — decide between fresh install (new random salt) and
    // legacy install (historical zero salt). A pre-existing database means
    // legacy.
    let legacy_db = data_dir.join("recap.db");
    if legacy_db.exists() {
        eprintln!(
            "WARNING: no salt file found but existing data is present; using the legacy \
             zero salt. Re-save your API keys to migrate them to per-install random salt \
             encryption."
        );
        return Ok([0u8; 32]);
    }

    let salt = EncryptionManager::generate_salt();
    write_salt_atomic(&salt_path, &salt)?;
    Ok(salt)
}

/// Write the salt via a temp file + rename so a crash mid-write cannot leave a
/// partial (corrupt) salt.bin (audit #49).
fn write_salt_atomic(salt_path: &Path, salt: &[u8; 32]) -> Result<()> {
    let tmp = salt_path.with_extension("bin.tmp");
    std::fs::write(&tmp, salt)?;
    std::fs::rename(&tmp, salt_path)?;
    Ok(())
}

impl StorageManager {
    pub fn new(data_dir: PathBuf, passphrase: &str) -> Result<Self> {
        let salt = load_or_create_salt(&data_dir)?;
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

    pub fn change_passphrase(&self, _new_passphrase: &str) -> Result<()> {
        // Audit #14: swapping the in-memory key without re-encrypting stored
        // data permanently bricks every saved API key (they were encrypted with
        // the old key, which is then discarded). Until full re-encryption is
        // implemented, refuse the operation instead of destroying credentials.
        Err(RecapError::Encryption(
            "change_passphrase is not implemented: stored data must be re-encrypted with the new key first; refusing to swap the key and lose saved API keys".to_string(),
        ))
    }

    pub fn get_data_dir(&self) -> &Path {
        &self.data_dir
    }

    pub fn get_encryption(&self) -> &EncryptionManager {
        &self.encryption
    }

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

    pub fn update_sync_status(&self, id: Uuid, sync_status: &SyncStatus) -> Result<()> {
        self.db.update_sync_status(id, sync_status)
    }

    pub fn list_meetings(&self, limit: i64, offset: i64) -> Result<Vec<Meeting>> {
        self.db.list_meetings(limit, offset)
    }

    pub fn delete_meeting(&self, id: Uuid) -> Result<()> {
        // Audit #12: a privacy-first delete must erase the recording too, not
        // just the DB rows. Capture the audio path before deleting, then remove
        // the file — but only when it lives inside the managed data directory.
        let audio_path = self.db.get_meeting(id)?.map(|m| m.audio_file_path);
        self.db.delete_meeting(id)?;
        if let Some(path) = audio_path {
            self.remove_managed_audio(&path);
        }
        Ok(())
    }

    /// Best-effort removal of a meeting audio file, confined to the data dir so
    /// a crafted audio_file_path can never delete an arbitrary file.
    fn remove_managed_audio(&self, path: &Path) {
        let canonical = match std::fs::canonicalize(path) {
            Ok(p) => p,
            Err(_) => return, // already gone / unreadable — nothing to remove
        };
        let data_dir = match std::fs::canonicalize(&self.data_dir) {
            Ok(d) => d,
            Err(_) => return,
        };
        if canonical.starts_with(&data_dir) {
            if let Err(e) = std::fs::remove_file(&canonical) {
                if e.kind() != std::io::ErrorKind::NotFound {
                    eprintln!("delete_meeting: failed to remove audio {}: {}", canonical.display(), e);
                }
            }
        }
    }

    pub fn search_transcripts(&self, query: &str, limit: i64) -> Result<Vec<(Uuid, String)>> {
        self.db.search_transcripts(query, limit)
    }

    // Transcript / summary / action item / decision operations
    pub fn save_transcript_segments(&self, meeting_id: Uuid, segments: &[TranscriptSegment]) -> Result<()> {
        self.db.save_transcript_segments(meeting_id, segments)
    }

    pub fn get_transcript_segments(&self, meeting_id: Uuid) -> Result<Vec<TranscriptSegment>> {
        self.db.get_transcript_segments(meeting_id)
    }

    pub fn save_summary(&self, meeting_id: Uuid, content: &str, generation_mode: &str, model_used: Option<&str>) -> Result<()> {
        self.db.save_summary(meeting_id, content, generation_mode, model_used)
    }

    pub fn get_latest_summary(&self, meeting_id: Uuid) -> Result<Option<Summary>> {
        self.db.get_latest_summary(meeting_id)
    }

    pub fn save_action_items(&self, meeting_id: Uuid, items: &[String]) -> Result<()> {
        self.db.save_action_items(meeting_id, items)
    }

    pub fn get_action_items(&self, meeting_id: Uuid) -> Result<Vec<String>> {
        self.db.get_action_items(meeting_id)
    }

    pub fn save_decisions(&self, meeting_id: Uuid, decisions: &[String]) -> Result<()> {
        self.db.save_decisions(meeting_id, decisions)
    }

    pub fn get_decisions(&self, meeting_id: Uuid) -> Result<Vec<String>> {
        self.db.get_decisions(meeting_id)
    }

    pub fn list_prompt_templates(&self) -> Result<Vec<(String, Option<String>, bool)>> {
        self.db.list_prompt_templates()
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

    // File operations
    pub fn write_encrypted_file(&self, relative_path: &Path, data: &[u8]) -> Result<()> {
        self.file_store.write_encrypted_file(relative_path, data)
    }

    pub fn read_encrypted_file(&self, relative_path: &Path) -> Result<Vec<u8>> {
        self.file_store.read_encrypted_file(relative_path)
    }

    pub fn write_plain_file(&self, relative_path: &Path, data: &[u8]) -> Result<()> {
        self.file_store.write_plain_file(relative_path, data)
    }

    pub fn read_plain_file(&self, relative_path: &Path) -> Result<Vec<u8>> {
        self.file_store.read_plain_file(relative_path)
    }
}

// EncryptionManager is Clone via derive
