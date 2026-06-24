// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod audio_input;
mod config;
mod error;
mod storage;
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
}

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[tauri::command]
fn create_meeting(title: String, state: State<AppState>) -> Result<String> {
    use uuid::Uuid;

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
        model_used: "whisper-medium".to_string(),
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
        audio_source: AudioSource::SystemCapture,
        stt_provider: "whisper_local".to_string(),
        llm_provider: "llama_local".to_string(),
        prompt_template_used: None,
    };

    let storage = state.storage.lock().map_err(|_| {
        crate::error::RecapError::Storage("Failed to lock storage".to_string())
    })?;
    storage.create_meeting(&meeting)?;
    Ok(meeting.id.to_string())
}

#[tauri::command]
fn list_meetings(limit: i64, offset: i64, state: State<AppState>) -> Result<Vec<String>> {
    let storage = state.storage.lock().map_err(|_| {
        crate::error::RecapError::Storage("Failed to lock storage".to_string())
    })?;
    let meetings = storage.list_meetings(limit, offset)?;
    Ok(meetings.into_iter().map(|m| m.id.to_string()).collect())
}

#[tauri::command]
fn get_config(key: String, state: State<AppState>) -> Result<Option<String>> {
    let config = state.config.lock().map_err(|_| {
        crate::error::RecapError::Config("Failed to lock config".to_string())
    })?;
    config.get_string(&key)
}

#[tauri::command]
fn import_audio_file(path: String, state: State<AppState>) -> Result<String> {
    use crate::audio_input::AudioInputManager;
    use std::path::PathBuf;

    let storage = state.storage.lock().map_err(|_| {
        crate::error::RecapError::Storage("Failed to lock storage".to_string())
    })?;

    let inbox_path = storage.get_file_store().get_inbox_dir();
    drop(storage);

    let manager = AudioInputManager::new(state.storage.clone(), inbox_path);
    let dest_path = manager.import_file(&PathBuf::from(path))?;
    Ok(dest_path.to_string_lossy().to_string())
}

#[tauri::command]
fn get_supported_audio_formats() -> Vec<String> {
    crate::audio_input::get_supported_audio_formats()
}

fn main() {
    let data_dir = dirs::home_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("RecapData");

    let storage = StorageManager::new(data_dir.clone(), "default-passphrase")
        .expect("Failed to initialize storage");

    let db_path = data_dir.join("recap.db");
    let db = SqliteManager::new(&db_path).expect("Failed to initialize database");
    let config = ConfigManager::new(db);

    let app_state = AppState {
        storage: Arc::new(Mutex::new(storage)),
        config: Arc::new(Mutex::new(config)),
    };

    tauri::Builder::default()
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![
            greet,
            create_meeting,
            list_meetings,
            get_config,
            import_audio_file,
            get_supported_audio_formats
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
