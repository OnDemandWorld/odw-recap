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
/// record so it shows up in the library immediately. The STT/LLM providers come
/// from Settings (R-4) — hardcoded 'whisper_local' made every watch-folder and
/// upload import fail transcription with "not available yet".
pub fn import_discovered_file(
    storage: &Arc<Mutex<StorageManager>>,
    inbox_path: &Path,
    source_path: &Path,
    stt_provider: &str,
    llm_provider: &str,
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

    create_meeting_for_inbox_file(
        storage,
        &dest_path,
        &title,
        &ext,
        crate::storage::types::AudioSource::WatchFolder,
        stt_provider,
        llm_provider,
    )
}

/// Create the library meeting row for an inbox audio file (R-2): shared by the
/// watch-folder importer and the LAN HTTP upload server so an uploaded file
/// becomes a REAL meeting instead of an orphan file with a fake id.
pub fn create_meeting_for_inbox_file(
    storage: &Arc<Mutex<StorageManager>>,
    dest_path: &Path,
    title: &str,
    ext: &str,
    source: crate::storage::types::AudioSource,
    stt_provider: &str,
    llm_provider: &str,
) -> Result<String> {
    let size = std::fs::metadata(dest_path).map(|m| m.len() as i64).unwrap_or(0);
    let now = chrono::Utc::now().timestamp_millis();

    let meeting = crate::storage::types::Meeting {
        id: Uuid::new_v4(),
        title: Some(title.to_string()),
        started_at: now,
        ended_at: None,
        duration_seconds: None,
        status: crate::storage::types::MeetingStatus::Recording,
        audio_file_path: dest_path.to_path_buf(),
        audio_format: ext.to_string(),
        audio_size_bytes: size,
        model_used: "unknown".to_string(),
        tags: vec![match source {
            crate::storage::types::AudioSource::WatchFolder => "watch_folder",
            _ => "phone_upload",
        }
        .to_string()],
        folder_id: None,
        created_at: now,
        updated_at: now,
        sync_status: crate::storage::types::SyncStatus::NotSynced,
        meeting_type: Some("team_meeting".to_string()),
        location: None,
        participants: Vec::new(),
        language: "en".to_string(),
        topic: None,
        audio_source: source,
        stt_provider: stt_provider.to_string(),
        llm_provider: llm_provider.to_string(),
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
        // Audit #7: reject symlinks before any copy. Watch folders are often
        // cloud-synced; a planted link (e.g. meeting.mp3 -> ~/.ssh/id_rsa)
        // would otherwise have its target's bytes copied into the library and
        // uploaded to cloud STT. symlink_metadata does not follow the link.
        reject_symlink(path)?;

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
    pub fn start_watch_folder<F>(
        &mut self,
        watch_path: PathBuf,
        stt_provider: String,
        llm_provider: String,
        mut on_imported: F,
    ) -> Result<()>
    where
        F: FnMut(String, String) + Send + 'static,
    {
        // Stop any previous watcher first.
        self.stop_watch_folder();

        // Audit #8: watching the inbox (or a folder containing/inside it) makes
        // every import copy a new UUID file back into the watched directory,
        // retriggering Create events in an unbounded self-import loop. Reject it.
        if watch_conflicts_with_inbox(&watch_path, &self.inbox_path) {
            return Err(RecapError::AudioInput(
                "Watch folder cannot be the inbox, contain it, or live inside it (would cause an infinite self-import loop)".to_string(),
            ));
        }

        let monitor = watch_folder_monitor::WatchFolderMonitor::new(watch_path)?;
        let storage = self.storage.clone();
        let inbox = self.inbox_path.clone();

        let watcher = monitor.watch_with_callback(move |path| {
            match import_discovered_file(&storage, &inbox, &path, &stt_provider, &llm_provider) {
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
    pub fn start_http_server(
        &mut self,
        port: u16,
        stt_provider: String,
        llm_provider: String,
    ) -> Result<String> {
        if self.http_server.is_some() {
            return Err(RecapError::AudioInput(
                "HTTP upload server is already running".to_string(),
            ));
        }
        let server = http_upload_server::HttpUploadServer::new(
            port,
            self.inbox_path.clone(),
            self.storage.clone(),
            stt_provider,
            llm_provider,
        )?;
        let url = server.upload_url();
        let token = server.token().to_string();

        // Audit #9: bind synchronously so a busy port (or other bind failure)
        // propagates to the caller now, rather than being eprintln'd inside a
        // detached task while the UI reports a live URL for a dead server.
        let addr = std::net::SocketAddr::from(([0, 0, 0, 0], port));
        let std_listener = std::net::TcpListener::bind(addr)
            .map_err(|e| RecapError::AudioInput(format!("Failed to bind HTTP server: {}", e)))?;
        std_listener
            .set_nonblocking(true)
            .map_err(|e| RecapError::AudioInput(format!("Failed to configure HTTP server: {}", e)))?;

        let handle = tauri::async_runtime::spawn(async move {
            let listener = match tokio::net::TcpListener::from_std(std_listener) {
                Ok(l) => l,
                Err(e) => {
                    eprintln!("HTTP upload server failed to register listener: {}", e);
                    return;
                }
            };
            if let Err(e) = server.run_with_listener(listener).await {
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

/// Audit #7: error when `path` is a symlink (or does not exist). Uses
/// symlink_metadata so the link itself is inspected, never its target.
fn reject_symlink(path: &Path) -> Result<()> {
    match std::fs::symlink_metadata(path) {
        Ok(m) if m.file_type().is_symlink() => Err(RecapError::AudioInput(format!(
            "Refusing to import symlink: {}",
            path.display()
        ))),
        Ok(_) => Ok(()),
        Err(_) => Err(RecapError::AudioInput(format!(
            "File does not exist: {}",
            path.display()
        ))),
    }
}

/// Audit #8: true when watching `watch_path` would re-import the inbox's own
/// output — i.e. the two paths are equal, or one contains the other. Compares
/// canonical paths where possible, falling back to the lexical paths.
fn watch_conflicts_with_inbox(watch_path: &Path, inbox_path: &Path) -> bool {
    let w = std::fs::canonicalize(watch_path).unwrap_or_else(|_| watch_path.to_path_buf());
    let i = std::fs::canonicalize(inbox_path).unwrap_or_else(|_| inbox_path.to_path_buf());
    w == i || i.starts_with(&w) || w.starts_with(&i)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn watch_conflicts_detects_inbox_overlap() {
        let tmp = tempfile::tempdir().unwrap();
        let inbox = tmp.path().join("inbox");
        std::fs::create_dir_all(&inbox).unwrap();
        let other = tmp.path().join("recordings");
        std::fs::create_dir_all(&other).unwrap();

        // Same dir, ancestor of inbox, and a child of inbox all conflict.
        assert!(watch_conflicts_with_inbox(&inbox, &inbox));
        assert!(watch_conflicts_with_inbox(tmp.path(), &inbox));
        let inside = inbox.join("sub");
        std::fs::create_dir_all(&inside).unwrap();
        assert!(watch_conflicts_with_inbox(&inside, &inbox));
        // A separate folder is fine.
        assert!(!watch_conflicts_with_inbox(&other, &inbox));
    }

    #[cfg(unix)]
    #[test]
    fn reject_symlink_blocks_links_but_allows_regular_files() {
        let tmp = tempfile::tempdir().unwrap();
        let target = tmp.path().join("secret.txt");
        std::fs::write(&target, b"not audio").unwrap();
        let link = tmp.path().join("meeting.mp3");
        std::os::unix::fs::symlink(&target, &link).unwrap();

        assert!(reject_symlink(&link).is_err(), "symlink must be rejected");
        assert!(reject_symlink(&target).is_ok(), "regular file must pass");
        assert!(reject_symlink(&tmp.path().join("missing.mp3")).is_err());
    }
}
