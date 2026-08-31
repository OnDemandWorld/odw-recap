use crate::error::{RecapError, Result};
use std::path::{Path, PathBuf};
use uuid::Uuid;

pub mod file_importer;
pub mod http_upload_server;
pub mod watch_folder_monitor;

/// Audio extensions accepted by the import pipeline.
pub const SUPPORTED_AUDIO_FORMATS: &[&str] = &["m4a", "wav", "mp3", "ogg", "webm", "flac"];

pub struct AudioInputManager {
    inbox_path: PathBuf,
    // Watch-folder and HTTP-upload support are wired up in a later milestone.
    #[allow(dead_code)]
    watch_monitor: Option<watch_folder_monitor::WatchFolderMonitor>,
    #[allow(dead_code)]
    http_server: Option<http_upload_server::HttpUploadServer>,
}

impl AudioInputManager {
    pub fn new(inbox_path: PathBuf) -> Self {
        Self {
            inbox_path,
            watch_monitor: None,
            http_server: None,
        }
    }

    #[allow(dead_code)]
    pub fn set_inbox_path(&mut self, path: PathBuf) {
        self.inbox_path = path;
    }

    #[allow(dead_code)]
    pub fn get_inbox_path(&self) -> &Path {
        &self.inbox_path
    }

    pub fn is_supported_format(path: &Path) -> bool {
        match path.extension() {
            Some(ext) => {
                let ext = ext.to_string_lossy().to_lowercase();
                SUPPORTED_AUDIO_FORMATS.contains(&ext.as_str())
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

        if !Self::is_supported_format(path) {
            return Err(RecapError::AudioInput(format!(
                "Unsupported audio format: {}. Supported: M4A, WAV, MP3, OGG, WebM, FLAC",
                path.display()
            )));
        }

        Ok(())
    }

    /// Copy a source file into the inbox under a unique name and return the
    /// destination path together with the file size.
    pub fn import_file(&self, source_path: &Path) -> Result<ImportedFile> {
        self.validate_file(source_path)?;

        let ext = source_path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_lowercase())
            .unwrap_or_else(|| "opus".to_string());
        let uuid = Uuid::new_v4();
        let dest_path = self.inbox_path.join(format!("{}.{}", uuid, ext));

        std::fs::create_dir_all(&self.inbox_path)?;
        std::fs::copy(source_path, &dest_path)?;

        let size_bytes = std::fs::metadata(&dest_path)?.len();

        Ok(ImportedFile {
            path: dest_path,
            extension: ext,
            size_bytes,
            original_name: source_path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default(),
        })
    }

    #[allow(dead_code)]
    pub fn start_watch_folder(&mut self) -> Result<()> {
        let monitor = watch_folder_monitor::WatchFolderMonitor::new(self.inbox_path.clone())?;
        self.watch_monitor = Some(monitor);
        Ok(())
    }

    #[allow(dead_code)]
    pub fn start_http_server(&mut self, port: u16) -> Result<()> {
        let server = http_upload_server::HttpUploadServer::new(port, self.inbox_path.clone())?;
        self.http_server = Some(server);
        Ok(())
    }
}

/// Metadata about a file that was copied into the inbox.
pub struct ImportedFile {
    pub path: PathBuf,
    pub extension: String,
    pub size_bytes: u64,
    pub original_name: String,
}
