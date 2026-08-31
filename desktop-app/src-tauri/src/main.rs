// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod audio_capture;
mod audio_input;
mod config;
mod error;
mod prompt_manager;
mod storage;
mod summarization;
mod sync;
mod transcription;

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use tauri::State;

use crate::audio_input::{AudioInputManager, SUPPORTED_AUDIO_FORMATS};
use crate::config::ConfigManager;
use crate::error::{RecapError, Result};
use crate::storage::sqlite_manager::SqliteManager;
use crate::storage::types::*;
use crate::storage::StorageManager;

struct AppState {
    storage: Arc<Mutex<StorageManager>>,
    config: Arc<Mutex<ConfigManager>>,
}

/// Resolve the directory where Recap stores its database, audio, and keys.
fn resolve_data_dir() -> PathBuf {
    dirs::data_local_dir()
        .or_else(dirs::home_dir)
        .unwrap_or_else(|| PathBuf::from("."))
        .join("Recap")
}

/// Get the passphrase used to unlock local encryption.
///
/// Order of precedence:
/// 1. `RECAP_PASSPHRASE` environment variable (for users who manage their own secret)
/// 2. A per-installation random passphrase stored in `<data_dir>/.passphrase`
///    with owner-only permissions.
///
/// TODO: replace the file-stored passphrase with a real unlock flow (user
/// prompt + OS keychain) — see IMPROVEMENT_PLAN.md.
fn load_or_create_passphrase(data_dir: &Path) -> Result<String> {
    if let Ok(passphrase) = std::env::var("RECAP_PASSPHRASE") {
        if !passphrase.is_empty() {
            return Ok(passphrase);
        }
    }

    let passphrase_path = data_dir.join(".passphrase");
    if passphrase_path.exists() {
        let passphrase = std::fs::read_to_string(&passphrase_path)?;
        let passphrase = passphrase.trim().to_string();
        if !passphrase.is_empty() {
            return Ok(passphrase);
        }
    }

    std::fs::create_dir_all(data_dir)?;
    let random: String = {
        use rand::{distributions::Alphanumeric, Rng};
        rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(48)
            .map(char::from)
            .collect()
    };

    #[cfg(unix)]
    {
        use std::io::Write;
        use std::os::unix::fs::OpenOptionsExt;
        let mut file = std::fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .mode(0o600)
            .open(&passphrase_path)?;
        file.write_all(random.as_bytes())?;
    }
    #[cfg(not(unix))]
    {
        std::fs::write(&passphrase_path, random.as_bytes())?;
    }

    Ok(random)
}

fn lock_storage(storage: &Arc<Mutex<StorageManager>>) -> Result<std::sync::MutexGuard<'_, StorageManager>> {
    storage.lock().map_err(|_| {
        RecapError::Storage("Failed to lock storage (poisoned mutex)".to_string())
    })
}

fn lock_config(config: &Arc<Mutex<ConfigManager>>) -> Result<std::sync::MutexGuard<'_, ConfigManager>> {
    config.lock().map_err(|_| {
        RecapError::Config("Failed to lock config (poisoned mutex)".to_string())
    })
}

#[tauri::command]
fn create_meeting(title: String, state: State<AppState>) -> Result<String> {
    use uuid::Uuid;

    let now = chrono::Utc::now().timestamp_millis();
    let meeting = Meeting {
        id: Uuid::new_v4(),
        title: Some(title),
        started_at: now,
        ended_at: None,
        duration_seconds: None,
        status: MeetingStatus::Recording,
        audio_file_path: PathBuf::new(),
        audio_format: "opus".to_string(),
        audio_size_bytes: 0,
        model_used: String::new(),
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
        audio_source: AudioSource::SystemCapture,
        stt_provider: "whisper_local".to_string(),
        llm_provider: "llama_local".to_string(),
        prompt_template_used: None,
    };

    let meeting_id = meeting.id;
    lock_storage(&state.storage)?.create_meeting(&meeting)?;
    Ok(meeting_id.to_string())
}

#[tauri::command]
fn list_meetings(limit: i64, offset: i64, state: State<AppState>) -> Result<Vec<Meeting>> {
    lock_storage(&state.storage)?.list_meetings(limit, offset)
}

#[tauri::command]
fn get_meeting(id: String, state: State<AppState>) -> Result<Option<Meeting>> {
    let meeting_id = uuid::Uuid::parse_str(&id)
        .map_err(|e| RecapError::Storage(format!("Invalid meeting id: {}", e)))?;
    lock_storage(&state.storage)?.get_meeting(meeting_id)
}

#[tauri::command]
fn get_config(key: String, state: State<AppState>) -> Result<Option<String>> {
    lock_config(&state.config)?.get_string(&key)
}

#[tauri::command]
fn set_config(key: String, value: String, state: State<AppState>) -> Result<()> {
    lock_config(&state.config)?.set_string(&key, &value)
}

#[tauri::command]
fn save_api_key(provider: String, api_key: String, state: State<AppState>) -> Result<()> {
    lock_storage(&state.storage)?.save_api_key(&provider, &api_key)
}

#[tauri::command]
fn has_api_key(provider: String, state: State<AppState>) -> Result<bool> {
    Ok(lock_storage(&state.storage)?.get_api_key(&provider)?.is_some())
}

#[tauri::command]
fn import_audio_file(path: String, state: State<AppState>) -> Result<String> {
    use uuid::Uuid;

    let inbox_path = {
        let storage = lock_storage(&state.storage)?;
        storage.get_file_store().get_inbox_dir()
    };

    let manager = AudioInputManager::new(inbox_path);
    let imported = manager.import_file(Path::new(&path))?;

    // Create the meeting record for the imported file so it shows up in the
    // library with meaningful metadata.
    let now = chrono::Utc::now().timestamp_millis();
    let title = Path::new(&imported.original_name)
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "Imported recording".to_string());

    let meeting = Meeting {
        id: Uuid::new_v4(),
        title: Some(title),
        started_at: now,
        ended_at: None,
        duration_seconds: None,
        status: MeetingStatus::Processing,
        audio_file_path: imported.path.clone(),
        audio_format: imported.extension,
        audio_size_bytes: imported.size_bytes as i64,
        model_used: String::new(),
        tags: Vec::new(),
        folder_id: None,
        created_at: now,
        updated_at: now,
        sync_status: SyncStatus::NotSynced,
        meeting_type: None,
        location: None,
        participants: Vec::new(),
        language: "en".to_string(),
        topic: None,
        audio_source: AudioSource::FileImport,
        stt_provider: String::new(),
        llm_provider: String::new(),
        prompt_template_used: None,
    };

    let meeting_id = meeting.id;
    lock_storage(&state.storage)?.create_meeting(&meeting)?;

    Ok(meeting_id.to_string())
}

#[tauri::command]
fn get_supported_audio_formats() -> Vec<String> {
    SUPPORTED_AUDIO_FORMATS.iter().map(|s| s.to_string()).collect()
}

#[tauri::command]
fn get_transcript_segments(meeting_id: String, state: State<AppState>) -> Result<Vec<TranscriptSegment>> {
    let id = uuid::Uuid::parse_str(&meeting_id)
        .map_err(|e| RecapError::Storage(format!("Invalid meeting id: {}", e)))?;
    lock_storage(&state.storage)?.get_transcript_segments(id)
}

#[tauri::command]
fn get_summary(meeting_id: String, state: State<AppState>) -> Result<Option<Summary>> {
    let id = uuid::Uuid::parse_str(&meeting_id)
        .map_err(|e| RecapError::Storage(format!("Invalid meeting id: {}", e)))?;
    lock_storage(&state.storage)?.get_summary(id)
}

#[tauri::command]
fn search_transcripts(query: String, limit: i64, state: State<AppState>) -> Result<Vec<(String, String)>> {
    let results = lock_storage(&state.storage)?.search_transcripts(&query, limit)?;
    Ok(results.into_iter().map(|(id, text)| (id.to_string(), text)).collect())
}

/// Availability info for a single provider, surfaced in the settings UI.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ProviderStatus {
    pub name: String,
    pub available: bool,
    pub models: Vec<String>,
}

#[tauri::command]
async fn list_stt_providers(state: State<'_, AppState>) -> Result<Vec<ProviderStatus>> {
    use crate::transcription::STTRouter;

    let credentials = {
        let storage = lock_storage(&state.storage)?;
        let config = lock_config(&state.config)?;
        collect_credentials(&storage, &config)?
    };

    let router = STTRouter::with_all_providers(String::new(), &credentials);
    let mut statuses = Vec::new();
    for name in router.available_providers() {
        let Some(provider) = router.get_provider(&name) else { continue };
        statuses.push(ProviderStatus {
            name: provider.name().to_string(),
            available: provider.is_available().await.unwrap_or(false),
            models: provider.supported_models(),
        });
    }
    statuses.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(statuses)
}

#[tauri::command]
async fn list_llm_providers(state: State<'_, AppState>) -> Result<Vec<ProviderStatus>> {
    use crate::summarization::LLMRouter;

    let credentials = {
        let storage = lock_storage(&state.storage)?;
        let config = lock_config(&state.config)?;
        collect_credentials(&storage, &config)?
    };

    let router = LLMRouter::with_all_providers(String::new(), &credentials);
    let mut statuses = Vec::new();
    for name in router.available_providers() {
        let Some(provider) = router.get_provider(&name) else { continue };
        statuses.push(ProviderStatus {
            name: provider.name().to_string(),
            available: provider.is_available().await.unwrap_or(false),
            models: provider.supported_models(),
        });
    }
    statuses.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(statuses)
}

#[tauri::command]
fn list_prompt_templates(state: State<AppState>) -> Result<Vec<crate::prompt_manager::PromptTemplate>> {
    crate::prompt_manager::PromptManager::new(state.storage.clone()).list_templates()
}

#[tauri::command]
fn save_prompt_template(name: String, content: String, state: State<AppState>) -> Result<()> {
    let template = crate::prompt_manager::PromptTemplate::new(name, String::new(), content);
    crate::prompt_manager::PromptManager::new(state.storage.clone()).save_template(&template)
}

/// Providers for which we look up an encrypted API key in storage.
const API_KEY_PROVIDERS: &[&str] = &[
    "openai",
    "anthropic",
    "deepgram",
    "assemblyai",
    "google",
    "google_stt",
    "azure_speech",
    "azure_openai",
];

/// Config keys forwarded to provider routers as credentials.
const CONFIG_CREDENTIAL_KEYS: &[&str] = &[
    "aws_access_key_id",
    "aws_secret_access_key",
    "aws_region",
    "azure_region",
    "azure_openai_endpoint",
    "azure_openai_deployment",
    "ollama_url",
    "whisper_model_path",
    "llama_model_path",
];

/// Collect provider credentials from encrypted storage and config.
fn collect_credentials(
    storage: &StorageManager,
    config: &ConfigManager,
) -> Result<std::collections::HashMap<String, String>> {
    let mut credentials = std::collections::HashMap::new();
    for provider in API_KEY_PROVIDERS {
        if let Some(key) = storage.get_api_key(provider)? {
            credentials.insert(provider.to_string(), key);
        }
    }
    for key in CONFIG_CREDENTIAL_KEYS {
        if let Some(value) = config.get_string(key)? {
            credentials.insert(key.to_string(), value);
        }
    }
    Ok(credentials)
}

/// Transcribe a meeting's audio file with the configured STT provider and
/// store the resulting segments. Returns the number of segments stored.
#[tauri::command]
async fn transcribe_meeting(meeting_id: String, state: State<'_, AppState>) -> Result<usize> {
    use crate::transcription::{STTRouter, TranscriptionConfig, TranscriptionManager};

    // Gather everything we need without holding locks across awaits.
    let (meeting, provider_name, credentials) = {
        let storage = lock_storage(&state.storage)?;
        let config = lock_config(&state.config)?;
        let meeting_id = uuid::Uuid::parse_str(&meeting_id)
            .map_err(|e| RecapError::Storage(format!("Invalid meeting id: {}", e)))?;
        let meeting = storage
            .get_meeting(meeting_id)?
            .ok_or_else(|| RecapError::Storage("Meeting not found".to_string()))?;
        let provider_name = config.stt_provider()?;
        let credentials = collect_credentials(&storage, &config)?;
        (meeting, provider_name, credentials)
    };

    if meeting.audio_file_path.as_os_str().is_empty() {
        return Err(RecapError::Transcription(
            "Meeting has no audio file. Import an audio file first.".to_string(),
        ));
    }

    let router = STTRouter::with_all_providers(provider_name.clone(), &credentials);
    let manager = TranscriptionManager::new(router);
    let config = TranscriptionConfig {
        provider: Some(provider_name.clone()),
        language: Some(meeting.language.clone()),
        ..Default::default()
    };
    let result = manager.transcribe(&meeting.audio_file_path, config).await?;

    let segment_count = result.segments.len();
    let now = chrono::Utc::now().timestamp_millis();

    {
        let storage = lock_storage(&state.storage)?;
        for segment in &result.segments {
            let mut segment = segment.clone();
            segment.meeting_id = meeting.id;
            storage.insert_transcript_segment(&segment)?;
        }

        let mut updated = meeting.clone();
        updated.status = MeetingStatus::Completed;
        updated.stt_provider = provider_name;
        updated.updated_at = now;
        storage.update_meeting(&updated)?;
    }

    Ok(segment_count)
}

/// Summarize a meeting's transcript with the configured LLM provider (or the
/// rule-based fallback when no provider is available). Returns the summary text.
#[tauri::command]
async fn summarize_meeting(meeting_id: String, state: State<'_, AppState>) -> Result<String> {
    use crate::summarization::{LLMRouter, SummarizationConfig, SummarizationManager};

    let (meeting_id, provider_name, credentials) = {
        let storage = lock_storage(&state.storage)?;
        let config = lock_config(&state.config)?;
        let meeting_id = uuid::Uuid::parse_str(&meeting_id)
            .map_err(|e| RecapError::Storage(format!("Invalid meeting id: {}", e)))?;
        let _meeting = storage
            .get_meeting(meeting_id)?
            .ok_or_else(|| RecapError::Storage("Meeting not found".to_string()))?;
        let provider_name = config.llm_provider()?;
        let credentials = collect_credentials(&storage, &config)?;
        (meeting_id, provider_name, credentials)
    };

    let segments = lock_storage(&state.storage)?.get_transcript_segments(meeting_id)?;
    if segments.is_empty() {
        return Err(RecapError::Summarization(
            "No transcript found for this meeting. Run transcription first.".to_string(),
        ));
    }
    let transcript: Vec<String> = segments.iter().map(|s| s.text.clone()).collect();
    let text = transcript.join("\n");

    let router = LLMRouter::with_all_providers(provider_name.clone(), &credentials);
    let provider_available = match router.get_provider(&provider_name) {
        Some(provider) => provider.is_available().await.unwrap_or(false),
        None => false,
    };

    // Fall back to the local rule-based summarizer so the feature works even
    // without any LLM provider configured.
    let config = SummarizationConfig {
        provider: Some(provider_name.clone()),
        use_rule_based: !provider_available,
        ..Default::default()
    };

    let manager = SummarizationManager::new(router);
    let result = manager.summarize(&text, config).await?;

    let now = chrono::Utc::now().timestamp_millis();
    let summary = Summary {
        id: 0,
        meeting_id,
        content: result.summary.clone(),
        generation_mode: result.provider.clone(),
        model_used: None,
        created_at: now,
        updated_at: now,
        user_edited: false,
    };
    lock_storage(&state.storage)?.save_summary(&summary)?;

    Ok(result.summary)
}

fn main() {
    let data_dir = resolve_data_dir();
    let passphrase = load_or_create_passphrase(&data_dir)
        .expect("Failed to initialize local passphrase");

    let storage = StorageManager::new(data_dir.clone(), &passphrase)
        .expect("Failed to initialize storage");

    let db_path = data_dir.join("recap.db");
    let db = SqliteManager::new(&db_path).expect("Failed to initialize database");
    let config = ConfigManager::new(db);

    let app_state = AppState {
        storage: Arc::new(Mutex::new(storage)),
        config: Arc::new(Mutex::new(config)),
    };

    // Seed default prompt templates (idempotent; existing templates win).
    if let Err(e) =
        crate::prompt_manager::PromptManager::new(app_state.storage.clone())
            .initialize_default_templates()
    {
        eprintln!("Failed to initialize default prompt templates: {}", e);
    }

    tauri::Builder::default()
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![
            create_meeting,
            list_meetings,
            get_meeting,
            get_config,
            set_config,
            save_api_key,
            has_api_key,
            import_audio_file,
            get_supported_audio_formats,
            get_transcript_segments,
            get_summary,
            search_transcripts,
            transcribe_meeting,
            summarize_meeting,
            list_prompt_templates,
            save_prompt_template,
            list_stt_providers,
            list_llm_providers
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
