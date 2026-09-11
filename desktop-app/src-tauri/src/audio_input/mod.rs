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
    watch_watcher: Option<notify::RecommendedWatcher>,
    http_server: Option<HttpUploadServerHandle>,
}

/// Copy a discovered audio file into the inbox and create a matching meeting
/// record so it shows up in the library immediately.
pub fn import_discovered_file(
    storage: &Arc<Mutex<StorageManager>>,
    inbox_path: &Path,
    source_path: &Path,
) -> Result<String> {
    let manager = AudioInputManager::new(storage.clone(), inbox_path.to_path_buf());
    manager.validate_file(source_path)?;

    let ext = source_path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("m4a")
        .to_string();
    let dest_path = inbox_path.join(format!("{}.{}", Uuid::new_v4(), ext));
    std::fs::copy(source_path, &dest_path)?;

    let title = source_path
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "Watched folder recording".to_string());
    let size = std::fs::metadata(&dest_path).map(|m| m.len() as i64).unwrap_or(0);

    let meeting = crate::storage::types::Meeting {
        id: Uuid::new_v4(),
        title: Some(title.clone()),
        started_at: chrono::Utc::now().timestamp_millis(),
        ended_at: None,
        duration_seconds: None,
        status: crate::storage::types::MeetingStatus::Recording,
        audio_file_path: dest_path,
        audio_format: ext,
        audio_size_bytes: size,
        model_used: "unknown".to_string(),
        tags: vec!["watch_folder".to_string()],
        folder_id: None,
        created_at: chrono::Utc::now().timestamp_millis(),
        updated_at: chrono::Utc::now().timestamp_millis(),
        sync_status: crate::storage::types::SyncStatus::NotSynced,
        meeting_type: Some("team_meeting".to_string()),
        location: None,
        participants: Vec::new(),
        language: "en".to_string(),
        topic: None,
        audio_source: crate::storage::types::AudioSource::WatchFolder,
        stt_provider: "whisper_local".to_string(),
        llm_provider: "rule_based".to_string(),
        prompt_template_used: None,
    };
    let meeting_id = meeting.id.to_string();
    {
        let db = storage.lock().map_err(|_| {
            RecapError::Storage("Failed to lock storage for watch import".to_string())
        })?;
        db.create_meeting(&meeting)?;
    }
    Ok(meeting_id)
}

/// Handle to a running HTTP upload server, kept so it can be stopped and its
/// URL/token surfaced in the UI.
pub struct HttpUploadServerHandle {
    abort: tauri::async_runtime::JoinHandle<()>,
    pub url: String,
    pub token: String,
    pub port: u16,
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
            watch_watcher: None,
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

    /// Watch `watch_path` (typically the user's recordings folder, not the
    /// inbox) and auto-import every new audio file into the library.
    /// `on_imported` is invoked with the new meeting id + title so the UI can
    /// refresh. The watcher runs on notify's own thread; the initial
    /// `wait_for_file_stable` delay (a few seconds) before import is
    /// intentional, to avoid reading files that are still being written.
    pub fn start_watch_folder<F>(&mut self, watch_path: PathBuf, mut on_imported: F) -> Result<()>
    where
        F: FnMut(String, String) + Send + 'static,
    {
        // Stop any previous watcher first.
        self.stop_watch_folder();

        let monitor = watch_folder_monitor::WatchFolderMonitor::new(watch_path)?;
        let storage = self.storage.clone();
        let inbox = self.inbox_path.clone();

        let watcher = monitor.watch_with_callback(move |path| {
            match import_discovered_file(&storage, &inbox, &path) {
                Ok(meeting_id) => {
                    let title = path
                        .file_stem()
                        .map(|s| s.to_string_lossy().to_string())
                        .unwrap_or_default();
                    on_imported(meeting_id, title);
                }
                Err(e) => eprintln!("watch folder: failed to import {}: {}", path.display(), e),
            }
        })?;

        self.watch_watcher = Some(watcher);
        self.watch_monitor = Some(monitor);
        Ok(())
    }

    /// Stop an active watch-folder watcher. Returns true when one was active.
    pub fn stop_watch_folder(&mut self) -> bool {
        let had = self.watch_watcher.take().is_some();
        self.watch_monitor = None;
        had
    }

    pub fn watch_folder_active(&self) -> bool {
        self.watch_watcher.is_some()
    }

    /// Start the LAN upload server on a background task and return its
    /// user-facing URL (token included).
    pub fn start_http_server(&mut self, port: u16) -> Result<String> {
        if self.http_server.is_some() {
            return Err(RecapError::AudioInput(
                "HTTP upload server is already running".to_string(),
            ));
        }
        let server = http_upload_server::HttpUploadServer::new(port, self.inbox_path.clone())?;
        let url = server.upload_url();
        let token = server.token().to_string();

        let handle = tauri::async_runtime::spawn(async move {
            if let Err(e) = server.run().await {
                eprintln!("HTTP upload server stopped with error: {}", e);
            }
        });

        self.http_server = Some(HttpUploadServerHandle {
            abort: handle,
            url,
            token,
            port,
        });
        Ok(self.http_server.as_ref().expect("just set").url.clone())
    }

    /// Stop a running upload server, if any. Returns true when one was stopped.
    pub fn stop_http_server(&mut self) -> bool {
        match self.http_server.take() {
            Some(handle) => {
                handle.abort.abort();
                true
            }
            None => false,
        }
    }

    pub fn http_server_url(&self) -> Option<&str> {
        self.http_server.as_ref().map(|h| h.url.as_str())
    }

    pub fn http_server_token(&self) -> Option<&str> {
        self.http_server.as_ref().map(|h| h.token.as_str())
    }

    pub fn http_server_port(&self) -> Option<u16> {
        self.http_server.as_ref().map(|h| h.port)
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
