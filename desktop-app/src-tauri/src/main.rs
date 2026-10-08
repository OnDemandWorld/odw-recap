// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod audio_capture;
mod audio_input;
mod config;
mod error;
mod processing;
mod prompt_manager;
mod storage;
mod summarization;
mod sync;
mod transcription;

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tauri::State;

use crate::config::ConfigManager;
use crate::error::Result;
use crate::storage::sqlite_manager::SqliteManager;
use crate::storage::types::*;
use crate::storage::StorageManager;

struct AppState {
    storage: Arc<Mutex<StorageManager>>,
    config: Arc<Mutex<ConfigManager>>,
    audio: Arc<Mutex<crate::audio_input::AudioInputManager>>,
    sync: Arc<crate::sync::SyncEngine>,
}

/// Config key holding the team-server base URL (default localhost:8080).
const TEAM_SERVER_URL_KEY: &str = "team_server.url";
const TEAM_SERVER_URL_DEFAULT: &str = "http://localhost:8080";

/// Richer meeting representation for the library list: the UI needs titles
/// and status, not just opaque ids.
#[derive(serde::Serialize)]
struct MeetingSummary {
    id: String,
    title: String,
    status: String,
    meeting_type: String,
    duration_seconds: Option<i32>,
    created_at: i64,
    updated_at: i64,
}

impl MeetingSummary {
    fn from_meeting(m: &Meeting) -> Self {
        Self {
            id: m.id.to_string(),
            title: m.title.clone().unwrap_or_else(|| "Untitled meeting".to_string()),
            status: m.status.to_string(),
            meeting_type: m.meeting_type.clone().unwrap_or_default(),
            duration_seconds: m.duration_seconds,
            created_at: m.created_at,
            updated_at: m.updated_at,
        }
    }
}

fn lock_storage<'a>(
    state: &'a State<'a, AppState>,
) -> Result<std::sync::MutexGuard<'a, StorageManager>> {
    state
        .storage
        .lock()
        .map_err(|_| crate::error::RecapError::Storage("Failed to lock storage".to_string()))
}

fn lock_config<'a>(
    state: &'a State<'a, AppState>,
) -> Result<std::sync::MutexGuard<'a, ConfigManager>> {
    state
        .config
        .lock()
        .map_err(|_| crate::error::RecapError::Config("Failed to lock config".to_string()))
}

/// Settings keys persisted by the UI (aligned with ConfigManager helpers).
const CONFIG_STT_PROVIDER: &str = "stt_provider";
const CONFIG_LLM_PROVIDER: &str = "llm_provider";

#[tauri::command]
fn create_meeting(title: String, state: State<AppState>) -> Result<String> {
    use uuid::Uuid;

    // Read provider defaults so newly created meetings honor Settings.
    let (stt_provider, llm_provider) = {
        let config = lock_config(&state)?;
        (
            config
                .get_string(CONFIG_STT_PROVIDER)?
                .unwrap_or_else(|| "whisper_local".to_string()),
            config
                .get_string(CONFIG_LLM_PROVIDER)?
                .unwrap_or_else(|| "rule_based".to_string()),
        )
    };

    let meeting = Meeting {
        id: Uuid::new_v4(),
        title: Some(title),
        started_at: chrono::Utc::now().timestamp_millis(),
        ended_at: None,
        duration_seconds: None,
        status: MeetingStatus::Recording,
        audio_file_path: PathBuf::new(),
        audio_format: "opus".to_string(),
        audio_size_bytes: 0,
        model_used: "unknown".to_string(),
        tags: Vec::new(),
        folder_id: None,
        created_at: chrono::Utc::now().timestamp_millis(),
        updated_at: chrono::Utc::now().timestamp_millis(),
        sync_status: SyncStatus::NotSynced,
        meeting_type: Some("team_meeting".to_string()),
        location: None,
        participants: Vec::new(),
        language: "en".to_string(),
        topic: None,
        audio_source: AudioSource::FileImport,
        stt_provider,
        llm_provider,
        prompt_template_used: None,
    };

    let storage = lock_storage(&state)?;
    storage.create_meeting(&meeting)?;
    Ok(meeting.id.to_string())
}

#[tauri::command]
fn list_meetings(limit: i64, offset: i64, state: State<AppState>) -> Result<Vec<MeetingSummary>> {
    let storage = lock_storage(&state)?;
    let meetings = storage.list_meetings(limit, offset)?;
    Ok(meetings.iter().map(MeetingSummary::from_meeting).collect())
}

#[tauri::command]
fn get_meeting(id: String, state: State<AppState>) -> Result<Meeting> {
    let meeting_id = parse_meeting_id(&id)?;
    let storage = lock_storage(&state)?;
    storage
        .get_meeting(meeting_id)?
        .ok_or_else(|| crate::error::RecapError::Storage(format!("Meeting not found: {}", id)))
}

#[tauri::command]
fn delete_meeting(id: String, state: State<AppState>) -> Result<()> {
    let meeting_id = parse_meeting_id(&id)?;
    let storage = lock_storage(&state)?;
    storage.delete_meeting(meeting_id)
}

#[tauri::command]
fn get_transcript(id: String, state: State<AppState>) -> Result<Vec<TranscriptSegment>> {
    let meeting_id = parse_meeting_id(&id)?;
    let storage = lock_storage(&state)?;
    storage.get_transcript_segments(meeting_id)
}

#[tauri::command]
fn get_summary(id: String, state: State<AppState>) -> Result<Option<Summary>> {
    let meeting_id = parse_meeting_id(&id)?;
    let storage = lock_storage(&state)?;
    storage.get_latest_summary(meeting_id)
}

#[tauri::command]
fn get_action_items(id: String, state: State<AppState>) -> Result<Vec<String>> {
    let meeting_id = parse_meeting_id(&id)?;
    let storage = lock_storage(&state)?;
    storage.get_action_items(meeting_id)
}

#[tauri::command]
fn get_decisions(id: String, state: State<AppState>) -> Result<Vec<String>> {
    let meeting_id = parse_meeting_id(&id)?;
    let storage = lock_storage(&state)?;
    storage.get_decisions(meeting_id)
}

#[tauri::command]
fn start_watch_folder(
    path: String,
    app: tauri::AppHandle,
    state: State<AppState>,
) -> Result<String> {
    let watch_path = PathBuf::from(&path);
    if !watch_path.is_dir() {
        return Err(crate::error::RecapError::AudioInput(format!(
            "Not a folder: {}",
            path
        )));
    }

    let mut audio = state
        .audio
        .lock()
        .map_err(|_| crate::error::RecapError::AudioInput("Failed to lock audio manager".to_string()))?;
    audio.start_watch_folder(watch_path, move |meeting_id, title| {
        use tauri::Manager;
        if let Err(e) = app.emit_all("meeting-imported", serde_json::json!({
            "meeting_id": meeting_id,
            "title": title,
            "source": "watch_folder",
        })) {
            eprintln!("failed to emit meeting-imported event: {}", e);
        }
    })?;
    Ok(path)
}

#[tauri::command]
fn stop_watch_folder(state: State<AppState>) -> Result<bool> {
    let mut audio = state
        .audio
        .lock()
        .map_err(|_| crate::error::RecapError::AudioInput("Failed to lock audio manager".to_string()))?;
    Ok(audio.stop_watch_folder())
}

#[tauri::command]
fn watch_folder_status(state: State<AppState>) -> Result<bool> {
    let audio = state
        .audio
        .lock()
        .map_err(|_| crate::error::RecapError::AudioInput("Failed to lock audio manager".to_string()))?;
    Ok(audio.watch_folder_active())
}

#[tauri::command]
async fn process_meeting(
    id: String,
    state: State<'_, AppState>,
) -> Result<processing::ProcessOutcome> {
    let meeting_id = parse_meeting_id(&id)?;
    let storage = state.storage.clone();
    processing::process_meeting(&storage, meeting_id).await
}

#[tauri::command]
fn get_config(key: String, state: State<AppState>) -> Result<Option<String>> {
    let config = lock_config(&state)?;
    config.get_string(&key)
}

#[tauri::command]
fn set_config(key: String, value: String, state: State<AppState>) -> Result<()> {
    let config = lock_config(&state)?;
    config.set_string(&key, &value)
}

#[tauri::command]
fn save_api_key(provider: String, key: String, state: State<AppState>) -> Result<()> {
    if provider.trim().is_empty() {
        return Err(crate::error::RecapError::Config("Provider name is required".to_string()));
    }
    let storage = lock_storage(&state)?;
    storage.save_api_key(provider.trim(), &key)
}

#[tauri::command]
fn has_api_key(provider: String, state: State<AppState>) -> Result<bool> {
    let storage = lock_storage(&state)?;
    Ok(storage.get_api_key(&provider)?.is_some())
}

#[tauri::command]
fn save_prompt_template(
    name: String,
    content: String,
    meeting_type: Option<String>,
    state: State<AppState>,
) -> Result<()> {
    if name.trim().is_empty() || content.trim().is_empty() {
        return Err(crate::error::RecapError::Config(
            "Template name and content are required".to_string(),
        ));
    }
    let storage = lock_storage(&state)?;
    storage.save_prompt_template(name.trim(), &content, meeting_type.as_deref(), true)
}

#[tauri::command]
fn list_prompt_templates(state: State<AppState>) -> Result<Vec<PromptTemplateInfo>> {
    let storage = lock_storage(&state)?;
    let templates = storage.list_prompt_templates()?;
    Ok(templates
        .into_iter()
        .map(|(name, meeting_type, is_custom)| PromptTemplateInfo {
            name,
            meeting_type,
            is_custom,
        })
        .collect())
}

#[derive(serde::Serialize)]
struct PromptTemplateInfo {
    name: String,
    meeting_type: Option<String>,
    is_custom: bool,
}

#[tauri::command]
fn import_audio_file(path: String, state: State<AppState>) -> Result<ImportedMeeting> {
    use uuid::Uuid;

    // Snapshot inbox path + suggested title without holding any lock.
    let (inbox_path, title) = {
        let storage = lock_storage(&state)?;
        let title = std::path::Path::new(&path)
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "Imported recording".to_string());
        (storage.get_file_store().get_inbox_dir(), title)
    };

    // Copy the file into the managed inbox (validates the extension).
    let dest_path = {
        let manager =
            crate::audio_input::AudioInputManager::new(state.storage.clone(), inbox_path);
        manager.import_file(&PathBuf::from(&path))?
    };

    // Read provider defaults from Settings.
    let (stt_provider, llm_provider) = {
        let config = lock_config(&state)?;
        (
            config
                .get_string(CONFIG_STT_PROVIDER)?
                .unwrap_or_else(|| "whisper_local".to_string()),
            config
                .get_string(CONFIG_LLM_PROVIDER)?
                .unwrap_or_else(|| "rule_based".to_string()),
        )
    };

    // Create the library record pointing at the inbox copy so the import is
    // immediately visible.
    let now = chrono::Utc::now().timestamp_millis();
    let size = std::fs::metadata(&dest_path).map(|m| m.len() as i64).unwrap_or(0);
    let meeting = Meeting {
        id: Uuid::new_v4(),
        title: Some(title),
        started_at: now,
        ended_at: None,
        duration_seconds: None,
        status: MeetingStatus::Recording,
        audio_file_path: dest_path.clone(),
        audio_format: dest_path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("m4a")
            .to_string(),
        audio_size_bytes: size,
        model_used: "unknown".to_string(),
        tags: Vec::new(),
        folder_id: None,
        created_at: now,
        updated_at: now,
        sync_status: SyncStatus::NotSynced,
        meeting_type: Some("team_meeting".to_string()),
        location: None,
        participants: Vec::new(),
        language: "en".to_string(),
        topic: None,
        audio_source: AudioSource::FileImport,
        stt_provider,
        llm_provider,
        prompt_template_used: None,
    };
    {
        let storage = lock_storage(&state)?;
        storage.create_meeting(&meeting)?;
    }

    Ok(ImportedMeeting {
        meeting_id: meeting.id.to_string(),
        audio_path: dest_path.to_string_lossy().to_string(),
    })
}

#[derive(serde::Serialize)]
struct ImportedMeeting {
    meeting_id: String,
    audio_path: String,
}

#[tauri::command]
fn start_http_upload_server(state: State<AppState>) -> Result<UploadServerInfo> {
    let port = {
        let config = lock_config(&state)?;
        config.http_upload_server_port()?
    };

    let mut audio = state
        .audio
        .lock()
        .map_err(|_| crate::error::RecapError::AudioInput("Failed to lock audio manager".to_string()))?;
    let url = audio.start_http_server(port)?;
    let token = audio
        .http_server_token()
        .unwrap_or_default()
        .to_string();

    Ok(UploadServerInfo { url, token, port })
}

#[derive(serde::Serialize)]
struct UploadServerInfo {
    url: String,
    token: String,
    port: u16,
}

#[tauri::command]
fn stop_http_upload_server(state: State<AppState>) -> Result<bool> {
    let mut audio = state
        .audio
        .lock()
        .map_err(|_| crate::error::RecapError::AudioInput("Failed to lock audio manager".to_string()))?;
    Ok(audio.stop_http_server())
}

#[tauri::command]
fn http_upload_server_status(state: State<AppState>) -> Result<Option<String>> {
    let audio = state
        .audio
        .lock()
        .map_err(|_| crate::error::RecapError::AudioInput("Failed to lock audio manager".to_string()))?;
    Ok(audio.http_server_url().map(str::to_string))
}

#[tauri::command]
fn get_supported_audio_formats() -> Vec<String> {
    crate::audio_input::get_supported_audio_formats()
}

fn parse_meeting_id(id: &str) -> Result<uuid::Uuid> {
    uuid::Uuid::parse_str(id).map_err(|_| {
        crate::error::RecapError::Storage(format!("Invalid meeting id: {}", id))
    })
}

fn main() {
    let data_dir = dirs::home_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("RecapData");

    // SECURITY: the storage encryption passphrase is sourced from the
    // environment with intentionally NO hardcoded default. When
    // RECAP_ENCRYPTION_PASSPHRASE is unset we fall back to an empty passphrase
    // and emit a loud warning — a documented development-only convenience, not
    // a production default. Production deployments MUST set the variable.
    let encryption_passphrase =
        std::env::var("RECAP_ENCRYPTION_PASSPHRASE").unwrap_or_else(|_| {
            eprintln!(
                "WARNING: RECAP_ENCRYPTION_PASSPHRASE is not set; falling back to an \
                 empty passphrase. Set it in production — local data will not be \
                 protected by a meaningful encryption key."
            );
            String::new()
        });

    let storage = StorageManager::new(data_dir.clone(), &encryption_passphrase)
        .expect("Failed to initialize storage");
    let app_storage = Arc::new(Mutex::new(storage));

    let db_path = data_dir.join("recap.db");
    let db = SqliteManager::new(&db_path).expect("Failed to initialize database");
    let config = ConfigManager::new(db);

    let inbox_path = app_storage
        .lock()
        .expect("storage lock poisoned during startup")
        .get_file_store()
        .get_inbox_dir();
    let audio = crate::audio_input::AudioInputManager::new(
        app_storage.clone(),
        inbox_path,
    );

    let app_state = AppState {
        storage: app_storage,
        config: Arc::new(Mutex::new(config)),
        audio: Arc::new(Mutex::new(audio)),
        sync: Arc::new(crate::sync::SyncEngine::new(
            TEAM_SERVER_URL_DEFAULT.to_string(),
            None,
        )),
    };

    tauri::Builder::default()
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![
            greet,
            create_meeting,
            list_meetings,
            get_meeting,
            delete_meeting,
            get_transcript,
            get_summary,
            get_action_items,
            get_decisions,
            process_meeting,
            get_config,
            set_config,
            save_api_key,
            has_api_key,
            save_prompt_template,
            list_prompt_templates,
            import_audio_file,
            start_watch_folder,
            stop_watch_folder,
            watch_folder_status,
            start_http_upload_server,
            stop_http_upload_server,
            http_upload_server_status,
            get_supported_audio_formats,
            team_server_login,
            team_server_logout,
            team_server_status,
            sync_meeting_to_team_server
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

// ---------------------------------------------------------------------------
// Team-server sync (Recap → Vault / Loop, via team-server /sync)
// ---------------------------------------------------------------------------

fn team_server_url(config: &ConfigManager) -> Result<String> {
    Ok(config
        .get_string(TEAM_SERVER_URL_KEY)?
        .unwrap_or_else(|| TEAM_SERVER_URL_DEFAULT.to_string()))
}

#[derive(serde::Serialize)]
struct TeamServerStatus {
    logged_in: bool,
    server_url: String,
}

/// Authenticate against the team server (POST /auth/login → JWT).
#[tauri::command]
async fn team_server_login(
    email: String,
    password: String,
    server_url: Option<String>,
    state: State<'_, AppState>,
) -> Result<String> {
    // Remember the URL for later sessions (persisted via the config DB) and
    // rebuild the engine against it before authenticating.
    {
        let config = state
            .config
            .lock()
            .map_err(|e| crate::error::RecapError::Config(format!("config lock: {}", e)))?;
        let url = match server_url {
            Some(u) => u,
            None => team_server_url(&config)?,
        };
        config.set_string(TEAM_SERVER_URL_KEY, &url)?;
        state.sync.reconfigure(url, None)?;
    }
    // reqwest runs on the async runtime; storage/config locks are released.
    let token = state.sync.login(&email, &password).await?;
    Ok(token)
}

/// Drop the stored team-server session token.
#[tauri::command]
fn team_server_logout(state: State<AppState>) -> Result<()> {
    state.sync.logout()
}

/// Report login state + configured server URL for the UI.
#[tauri::command]
fn team_server_status(state: State<AppState>) -> Result<TeamServerStatus> {
    let config = state
        .config
        .lock()
        .map_err(|e| crate::error::RecapError::Config(format!("config lock: {}", e)))?;
    Ok(TeamServerStatus {
        logged_in: state.sync.is_logged_in()?,
        server_url: team_server_url(&config)?,
    })
}

/// Push a full meeting (metadata + transcript + summary + action items) to
/// the team server, which forwards to Vault (knowledge) and Loop (workflows).
/// The local `sync_status` column is updated to reflect the outcome.
#[tauri::command]
async fn sync_meeting_to_team_server(
    id: String,
    state: State<'_, AppState>,
) -> Result<String> {
    let meeting_id = parse_meeting_id(&id)?;

    // Gather the payload from local storage, then release the lock before
    // any network I/O (locks must never span await points).
    let (mut meeting, transcript, summary, action_items) = {
        let storage = lock_storage(&state)?;
        let meeting = storage
            .get_meeting(meeting_id)?
            .ok_or_else(|| {
                crate::error::RecapError::Storage(format!("meeting not found: {}", id))
            })?;
        let transcript = storage.get_transcript_segments(meeting_id)?;
        let summary = storage.get_latest_summary(meeting_id)?;
        let action_items = storage.get_action_items(meeting_id)?;
        (meeting, transcript, summary, action_items)
    };

    let mut meeting_data = std::collections::HashMap::new();
    meeting_data.insert(
        "title".to_string(),
        meeting.title.clone().unwrap_or_else(|| "Untitled meeting".to_string()),
    );
    if let Some(kind) = &meeting.meeting_type {
        meeting_data.insert("meeting_type".to_string(), kind.clone());
    }
    if let Some(secs) = meeting.duration_seconds {
        meeting_data.insert("duration_seconds".to_string(), secs.to_string());
    }
    meeting_data.insert("language".to_string(), meeting.language.clone());
    if !meeting.participants.is_empty() {
        meeting_data.insert(
            "participants".to_string(),
            meeting.participants.join(", "),
        );
    }

    let transcript_data = serde_json::json!({
        "segments": transcript
            .iter()
            .map(|s| serde_json::json!({
                "start_ms": s.start_ms,
                "end_ms": s.end_ms,
                "text": s.text,
                "speaker": s.speaker_id,
            }))
            .collect::<Vec<_>>(),
    });

    // NOTE: "action_items" is the trigger key for the Recap→Loop chain
    // (INTEGRATION_CONTRACT §3 / DEPLOYMENT.md) — keep the exact field name.
    let summary_data = serde_json::json!({
        "content": summary.as_ref().map(|s| s.content.clone()).unwrap_or_default(),
        "action_items": action_items,
    });

    // Persist the attempt before the network call.
    meeting.sync_status = SyncStatus::Syncing;
    {
        let storage = lock_storage(&state)?;
        storage.update_meeting(&meeting)?;
    }

    let result = state
        .sync
        .sync_meeting(&id, meeting_data, transcript_data, summary_data)
        .await;

    match result {
        Ok(resp) => {
            let status = if resp.status == "synced" {
                SyncStatus::Synced
            } else if resp.status == "partial" {
                // Forwarded with errors; the team-server queue worker retries.
                SyncStatus::Syncing
            } else {
                SyncStatus::Failed
            };
            let message = resp.message.clone();
            let status_str = resp.status.clone();
            meeting.sync_status = status;
            {
                let storage = lock_storage(&state)?;
                storage.update_meeting(&meeting)?;
            }
            Ok(format!("{}: {}", status_str, message))
        }
        Err(e) => {
            meeting.sync_status = SyncStatus::Failed;
            {
                let storage = lock_storage(&state)?;
                storage.update_meeting(&meeting)?;
            }
            Err(e)
        }
    }
}
