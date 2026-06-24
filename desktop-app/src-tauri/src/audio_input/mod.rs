use crate::error::{RecapError, Result};
use crate::storage::StorageManager;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use uuid::Uuid;

pub mod file_importer;
pub mod http_upload_server;
pub mod watch_folder_monitor;

pub struct AudioInputManager {
    storage: Arc<Mutex<StorageManager>>,
    inbox_path: PathBuf,
    supported_formats: Vec<String>,
    watch_monitor: Option<watch_folder_monitor::WatchFolderMonitor>,
    http_server: Option<http_upload_server::HttpUploadServer>,
}

impl AudioInputManager {
    pub fn new(storage: Arc<Mutex<StorageManager>>, inbox_path: PathBuf) -> Self {
        let supported_formats = vec![
            "m4a".to_string(),
            "wav".to_string(),
            "mp3".to_string(),
            "ogg".to_string(),
            "webm".to_string(),
            "flac".to_string(),
        ];

        Self {
            storage,
            inbox_path,
            supported_formats,
            watch_monitor: None,
            http_server: None,
        }
    }

    pub fn set_inbox_path(&mut self, path: PathBuf) {
        self.inbox_path = path;
    }

    pub fn get_inbox_path(&self) -> &Path {
        &self.inbox_path
    }

    pub fn is_supported_format(&self, path: &Path) -> bool {
        match path.extension() {
            Some(ext) => {
                let ext = ext.to_string_lossy().to_lowercase();
                self.supported_formats.contains(&ext.to_string())
            }
            None => false,
        }
    }

    pub fn validate_file(&self, path: &Path) -> Result<()> {
        if !path.exists() {
            return Err(RecapError::AudioInput(format!(
                "File does not exist: {}",
                path.display()
            )));
        }

        if !self.is_supported_format(path) {
            return Err(RecapError::AudioInput(format!(
                "Unsupported audio format: {}. Supported: M4A, WAV, MP3, OGG, WebM, FLAC",
                path.display()
            )));
        }

        Ok(())
    }

    pub fn import_file(&self, source_path: &Path) -> Result<PathBuf> {
        self.validate_file(source_path)?;

        let ext = source_path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("opus");
        let uuid = Uuid::new_v4();
        let dest_path = self.inbox_path.join(format!("{}.{}", uuid, ext));

        std::fs::copy(source_path, &dest_path)?;

        Ok(dest_path)
    }

    pub fn start_watch_folder(&mut self) -> Result<()> {
        let monitor = watch_folder_monitor::WatchFolderMonitor::new(self.inbox_path.clone())?;
        self.watch_monitor = Some(monitor);
        Ok(())
    }

    pub fn start_http_server(&mut self, port: u16) -> Result<()> {
        let server = http_upload_server::HttpUploadServer::new(port, self.inbox_path.clone())?;
        self.http_server = Some(server);
        Ok(())
    }
}

pub fn get_supported_audio_formats() -> Vec<String> {
    vec![
        "m4a".to_string(),
        "wav".to_string(),
        "mp3".to_string(),
        "ogg".to_string(),
        "webm".to_string(),
        "flac".to_string(),
    ]
}
