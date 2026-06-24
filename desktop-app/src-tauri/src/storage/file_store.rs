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

    pub fn write_encrypted_file(&self, relative_path: &Path, data: &[u8]) -> Result<()> {
        let full_path = self.data_dir.join(relative_path);
        if let Some(parent) = full_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let encrypted = self.encryption.encrypt(data)?;
        fs::write(full_path, encrypted)?;
        Ok(())
    }

    pub fn read_encrypted_file(&self, relative_path: &Path) -> Result<Vec<u8>> {
        let full_path = self.data_dir.join(relative_path);
        let encrypted = fs::read(full_path)?;
        self.encryption.decrypt(&encrypted)
    }

    pub fn write_plain_file(&self, relative_path: &Path, data: &[u8]) -> Result<()> {
        let full_path = self.data_dir.join(relative_path);
        if let Some(parent) = full_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(full_path, data)?;
        Ok(())
    }

    pub fn read_plain_file(&self, relative_path: &Path) -> Result<Vec<u8>> {
        let full_path = self.data_dir.join(relative_path);
        fs::read(full_path).map_err(|e| e.into())
    }

    pub fn exists(&self, relative_path: &Path) -> bool {
        self.data_dir.join(relative_path).exists()
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
