use crate::error::{RecapError, Result};
use crate::storage::sqlite_manager::SqliteManager;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub struct ConfigManager {
    db: SqliteManager,
    defaults: ConfigDefaults,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigDefaults {
    pub data_directory: PathBuf,
    pub watch_folder_enabled: bool,
    pub watch_folder_path: PathBuf,
    pub http_upload_server_enabled: bool,
    pub http_upload_server_port: u16,
    pub stt_provider: String,
    pub stt_fallback_provider: Option<String>,
    pub llm_provider: String,
    pub llm_model: String,
    pub prompt_template: String,
}

impl Default for ConfigDefaults {
    fn default() -> Self {
        let data_dir = dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("RecapData");

        Self {
            data_directory: data_dir.clone(),
            watch_folder_enabled: false,
            watch_folder_path: data_dir.join("inbox"),
            http_upload_server_enabled: false,
            http_upload_server_port: 8765,
            stt_provider: "whisper_local".to_string(),
            stt_fallback_provider: None,
            llm_provider: "llama_local".to_string(),
            llm_model: "llama-3-8b-instruct".to_string(),
            prompt_template: "standard_meeting_summary".to_string(),
        }
    }
}

impl ConfigManager {
    pub fn new(db: SqliteManager) -> Self {
        Self {
            db,
            defaults: ConfigDefaults::default(),
        }
    }

    pub fn get<T: serde::de::DeserializeOwned>(&self, key: &str) -> Result<Option<T>> {
        let value: Option<String> = self.db.get_config_value(key)?;
        match value {
            Some(v) => Ok(Some(serde_json::from_str(&v)?)),
            None => Ok(None),
        }
    }

    pub fn set<T: Serialize>(&self, key: &str, value: &T) -> Result<()> {
        let value_str = serde_json::to_string(value)?;
        self.db.set_config_value(key, &value_str)
    }

    pub fn get_string(&self, key: &str) -> Result<Option<String>> {
        self.db.get_config_value(key)
    }

    pub fn set_string(&self, key: &str, value: &str) -> Result<()> {
        self.db.set_config_value(key, value)
    }

    pub fn get_or_default<T: serde::de::DeserializeOwned + Default>(&self, key: &str) -> Result<T> {
        match self.get::<T>(key)? {
            Some(v) => Ok(v),
            None => Ok(T::default()),
        }
    }

    pub fn data_directory(&self) -> Result<PathBuf> {
        match self.get::<PathBuf>("data_directory")? {
            Some(dir) => Ok(dir),
            None => Ok(self.defaults.data_directory.clone()),
        }
    }

    pub fn watch_folder_path(&self) -> Result<PathBuf> {
        match self.get::<PathBuf>("watch_folder_path")? {
            Some(path) => Ok(path),
            None => Ok(self.defaults.watch_folder_path.clone()),
        }
    }

    pub fn http_upload_server_port(&self) -> Result<u16> {
        match self.get::<u16>("http_upload_server_port")? {
            Some(port) => Ok(port),
            None => Ok(self.defaults.http_upload_server_port),
        }
    }

    pub fn stt_provider(&self) -> Result<String> {
        match self.get::<String>("stt_provider")? {
            Some(provider) => Ok(provider),
            None => Ok(self.defaults.stt_provider.clone()),
        }
    }

    pub fn llm_provider(&self) -> Result<String> {
        match self.get::<String>("llm_provider")? {
            Some(provider) => Ok(provider),
            None => Ok(self.defaults.llm_provider.clone()),
        }
    }
}
