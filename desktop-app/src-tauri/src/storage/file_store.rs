use crate::error::{RecapError, Result};
use crate::storage::encryption_manager::EncryptionManager;
use std::fs;
use std::path::{Path, PathBuf};

pub struct FileStore {
    data_dir: PathBuf,
    encryption: EncryptionManager,
}

impl FileStore {
    pub fn new(data_dir: PathBuf, encryption: EncryptionManager) -> Self {
        Self {
            data_dir,
            encryption,
        }
    }

    pub fn init(&self) -> Result<()> {
        fs::create_dir_all(&self.data_dir)?;
        fs::create_dir_all(self.data_dir.join("audio"))?;
        fs::create_dir_all(self.data_dir.join("inbox"))?;
        fs::create_dir_all(self.data_dir.join("prompts"))?;
        Ok(())
    }

    /// Audit #48: confine caller-supplied relative paths to the data dir.
    /// Path::join with an absolute argument discards the base entirely, and
    /// '..' components escape it; reject both before joining.
    fn resolve(&self, relative_path: &Path) -> Result<PathBuf> {
        if relative_path.is_absolute() {
            return Err(RecapError::Storage(
                "absolute paths are not allowed in the file store".to_string(),
            ));
        }
        if relative_path
            .components()
            .any(|c| matches!(c, std::path::Component::ParentDir))
        {
            return Err(RecapError::Storage(
                "path traversal ('..') is not allowed in the file store".to_string(),
            ));
        }
        Ok(self.data_dir.join(relative_path))
    }

    pub fn write_encrypted_file(&self, relative_path: &Path, data: &[u8]) -> Result<()> {
        let full_path = self.resolve(relative_path)?;
        if let Some(parent) = full_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let encrypted = self.encryption.encrypt(data)?;
        fs::write(full_path, encrypted)?;
        Ok(())
    }

    pub fn read_encrypted_file(&self, relative_path: &Path) -> Result<Vec<u8>> {
        let full_path = self.resolve(relative_path)?;
        let encrypted = fs::read(full_path)?;
        self.encryption.decrypt(&encrypted)
    }

    pub fn write_plain_file(&self, relative_path: &Path, data: &[u8]) -> Result<()> {
        let full_path = self.resolve(relative_path)?;
        if let Some(parent) = full_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(full_path, data)?;
        Ok(())
    }

    pub fn read_plain_file(&self, relative_path: &Path) -> Result<Vec<u8>> {
        let full_path = self.resolve(relative_path)?;
        fs::read(full_path).map_err(|e| e.into())
    }

    pub fn exists(&self, relative_path: &Path) -> bool {
        self.resolve(relative_path).map(|p| p.exists()).unwrap_or(false)
    }

    pub fn get_data_dir(&self) -> &Path {
        &self.data_dir
    }

    pub fn get_inbox_dir(&self) -> PathBuf {
        self.data_dir.join("inbox")
    }

    pub fn get_audio_dir(&self) -> PathBuf {
        self.data_dir.join("audio")
    }

    pub fn get_prompts_dir(&self) -> PathBuf {
        self.data_dir.join("prompts")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Audit #48: relative paths are confined to the data dir; absolute paths and
    // '..' traversal are rejected.
    #[test]
    fn resolve_rejects_absolute_and_traversal() {
        let store = FileStore::new(PathBuf::from("/data"), EncryptionManager::new());
        assert!(store.resolve(Path::new("audio/x.m4a")).is_ok());
        assert!(store.resolve(Path::new("/etc/passwd")).is_err());
        assert!(store.resolve(Path::new("../escape")).is_err());
        assert!(store.resolve(Path::new("audio/../../escape")).is_err());
        assert_eq!(
            store.resolve(Path::new("audio/x.m4a")).unwrap(),
            PathBuf::from("/data/audio/x.m4a")
        );
    }
}
