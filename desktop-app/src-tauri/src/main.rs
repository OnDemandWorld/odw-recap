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
use std::sync::{Arc, Mutex, MutexGuard};
use tauri::State;

use crate::audio_input::{AudioInputManager, SUPPORTED_AUDIO_FORMATS};
use crate::config::ConfigManager;
use crate::error::{RecapError, Result};
use crate::storage::sqlite_manager::SqliteManager;
use crate::storage::types::*;
use crate::storage::StorageManager;

/// Locked/unlocked vault holding all application data managers.
///
/// Storage is only constructed after the vault is created (first run) or
/// unlocked (later runs), so encrypted data is never accessible while the
/// vault is locked.
struct VaultState {
    storage: Option<StorageManager>,
    config: Option<ConfigManager>,
}

struct AppState {
    vault: Arc<Mutex<VaultState>>,
    data_dir: PathBuf,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct VaultStatus {
    pub initialized: bool,
    pub locked: bool,
}

/// Resolve the directory where Recap stores its database, audio, and keys.
fn resolve_data_dir() -> PathBuf {
    dirs::data_local_dir()
        .or_else(dirs::home_dir)
        .unwrap_or_else(|| PathBuf::from("."))
        .join("Recap")
}

fn acquire_vault(vault: &Arc<Mutex<VaultState>>) -> Result<MutexGuard<'_, VaultState>> {
    vault
        .lock()
        .map_err(|_| RecapError::Vault("Failed to lock vault state (poisoned mutex)".to_string()))
}

fn storage_of<'a>(guard: &'a MutexGuard<'a, VaultState>) -> Result<&'a StorageManager> {
    guard.storage.as_ref().ok_or_else(|| {
        RecapError::Vault("Vault is locked — unlock it to continue".to_string())
    })
}

fn config_of<'a>(guard: &'a MutexGuard<'a, VaultState>) -> Result<&'a ConfigManager> {
    guard.config.as_ref().ok_or_else(|| {
        RecapError::Vault("Vault is locked — unlock it to continue".to_string())
    })
}

/// Build the config manager for the vault's database (a second SQLite
/// connection dedicated to configuration, which is not secret).
fn build_config(data_dir: &Path) -> Result<ConfigManager> {
    let db_path = data_dir.join("recap.db");
    let db = SqliteManager::new(&db_path)?;
    Ok(ConfigManager::new(db))
}

/// Seed default prompt templates (idempotent; existing templates win).
fn seed_default_templates(storage: &StorageManager) {
    if let Err(e) = crate::prompt_manager::PromptManager::new(storage).initialize_default_templates()
    {
        eprintln!("Failed to initialize default prompt templates: {}", e);
    }
}

fn validate_new_passphrase(passphrase: &str) -> Result<()> {
    if passphrase.len() < 8 {
        return Err(RecapError::Vault(
            "Passphrase must be at least 8 characters".to_string(),
        ));
    }
    Ok(())
}

// --- Vault commands ---------------------------------------------------------

#[tauri::command]
fn vault_status(state: State<AppState>) -> Result<VaultStatus> {
    let guard = acquire_vault(&state.vault)?;
    Ok(VaultStatus {
        initialized: storage::vault_initialized(&state.data_dir),
        locked: guard.storage.is_none(),
    })
}

/// First run: create the vault with the user's passphrase.
#[tauri::command]
fn initialize_vault(passphrase: String, confirm: String, state: State<AppState>) -> Result<()> {
    validate_new_passphrase(&passphrase)?;
    if passphrase != confirm {
        return Err(RecapError::Vault("Passphrases do not match".to_string()));
    }

    let mut guard = acquire_vault(&state.vault)?;
    if guard.storage.is_some() {
        return Err(RecapError::Vault("Vault is already initialized and unlocked".to_string()));
    }
    if storage::vault_initialized(&state.data_dir) {
        return Err(RecapError::Vault(
            "Vault is already initialized — unlock it instead".to_string(),
        ));
    }

    let storage = StorageManager::initialize(state.data_dir.clone(), &passphrase)?;
    seed_default_templates(&storage);
    guard.config = Some(build_config(&state.data_dir)?);
    guard.storage = Some(storage);
    Ok(())
}

/// Unlock an existing vault with the user's passphrase.
#[tauri::command]
fn unlock_vault(passphrase: String, state: State<AppState>) -> Result<()> {
    let mut guard = acquire_vault(&state.vault)?;
    if guard.storage.is_some() {
        return Ok(()); // already unlocked
    }
    if !storage::vault_initialized(&state.data_dir) {
        return Err(RecapError::Vault(
            "Vault is not initialized on this device — set one up first".to_string(),
        ));
    }

    let storage = StorageManager::open(state.data_dir.clone(), &passphrase)?;
    seed_default_templates(&storage);
    guard.config = Some(build_config(&state.data_dir)?);
    guard.storage = Some(storage);
    Ok(())
}

/// Lock the vault, dropping all in-memory key material.
#[tauri::command]
fn lock_vault(state: State<AppState>) -> Result<()> {
    let mut guard = acquire_vault(&state.vault)?;
    guard.storage = None;
    guard.config = None;
    Ok(())
}

/// Change the vault passphrase. All encrypted data is re-encrypted with the
/// new key in a single transaction (see StorageManager::change_passphrase).
#[tauri::command]
fn change_vault_passphrase(current: String, new: String, state: State<AppState>) -> Result<()> {
    validate_new_passphrase(&new)?;

    let guard = acquire_vault(&state.vault)?;
    let storage = storage_of(&guard)?;
    if !storage.get_encryption().verify_passphrase(&current)? {
        return Err(RecapError::Vault("Current passphrase is incorrect".to_string()));
    }
    storage.change_passphrase(&new)
}

// --- Meeting commands -------------------------------------------------------

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
    let guard = acquire_vault(&state.vault)?;
    storage_of(&guard)?.create_meeting(&meeting)?;
    Ok(meeting_id.to_string())
}

#[tauri::command]
fn list_meetings(limit: i64, offset: i64, state: State<AppState>) -> Result<Vec<Meeting>> {
    let guard = acquire_vault(&state.vault)?;
    storage_of(&guard)?.list_meetings(limit, offset)
}

#[tauri::command]
fn get_meeting(id: String, state: State<AppState>) -> Result<Option<Meeting>> {
    let meeting_id = uuid::Uuid::parse_str(&id)
        .map_err(|e| RecapError::Storage(format!("Invalid meeting id: {}", e)))?;
    let guard = acquire_vault(&state.vault)?;
    storage_of(&guard)?.get_meeting(meeting_id)
}

#[tauri::command]
fn get_config(key: String, state: State<AppState>) -> Result<Option<String>> {
    let guard = acquire_vault(&state.vault)?;
    config_of(&guard)?.get_string(&key)
}

#[tauri::command]
fn set_config(key: String, value: String, state: State<AppState>) -> Result<()> {
    let guard = acquire_vault(&state.vault)?;
    config_of(&guard)?.set_string(&key, &value)
}

#[tauri::command]
fn save_api_key(provider: String, api_key: String, state: State<AppState>) -> Result<()> {
    let guard = acquire_vault(&state.vault)?;
    storage_of(&guard)?.save_api_key(&provider, &api_key)
}

#[tauri::command]
fn has_api_key(provider: String, state: State<AppState>) -> Result<bool> {
    let guard = acquire_vault(&state.vault)?;
    Ok(storage_of(&guard)?.get_api_key(&provider)?.is_some())
}

#[tauri::command]
fn import_audio_file(path: String, state: State<AppState>) -> Result<String> {
    use uuid::Uuid;

    let inbox_path = {
        let guard = acquire_vault(&state.vault)?;
        storage_of(&guard)?.get_file_store().get_inbox_dir()
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
    let guard = acquire_vault(&state.vault)?;
    storage_of(&guard)?.create_meeting(&meeting)?;

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
    let guard = acquire_vault(&state.vault)?;
    storage_of(&guard)?.get_transcript_segments(id)
}

#[tauri::command]
fn get_summary(meeting_id: String, state: State<AppState>) -> Result<Option<Summary>> {
    let id = uuid::Uuid::parse_str(&meeting_id)
        .map_err(|e| RecapError::Storage(format!("Invalid meeting id: {}", e)))?;
    let guard = acquire_vault(&state.vault)?;
    storage_of(&guard)?.get_summary(id)
}

#[tauri::command]
fn search_transcripts(query: String, limit: i64, state: State<AppState>) -> Result<Vec<(String, String)>> {
    let guard = acquire_vault(&state.vault)?;
    let results = storage_of(&guard)?.search_transcripts(&query, limit)?;
    Ok(results.into_iter().map(|(id, text)| (id.to_string(), text)).collect())
}

// --- Provider commands ------------------------------------------------------

/// Availability info for a single provider, surfaced in the settings UI.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ProviderStatus {
    pub name: String,
    pub available: bool,
    pub models: Vec<String>,
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

/// Load credentials from the vault (single lock acquisition). Returns an
/// error when the vault is locked.
fn load_credentials(state: &State<AppState>) -> Result<std::collections::HashMap<String, String>> {
    let guard = acquire_vault(&state.vault)?;
    let storage = storage_of(&guard)?;
    let config = config_of(&guard)?;
    collect_credentials(storage, config)
}

#[tauri::command]
async fn list_stt_providers(state: State<'_, AppState>) -> Result<Vec<ProviderStatus>> {
    use crate::transcription::STTRouter;

    let credentials = load_credentials(&state)?;
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

    let credentials = load_credentials(&state)?;
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

// --- Prompt template commands -----------------------------------------------

#[tauri::command]
fn list_prompt_templates(state: State<AppState>) -> Result<Vec<crate::prompt_manager::PromptTemplate>> {
    let guard = acquire_vault(&state.vault)?;
    crate::prompt_manager::PromptManager::new(storage_of(&guard)?).list_templates()
}

#[tauri::command]
fn save_prompt_template(name: String, content: String, state: State<AppState>) -> Result<()> {
    let template = crate::prompt_manager::PromptTemplate::new(name, String::new(), content);
    let guard = acquire_vault(&state.vault)?;
    crate::prompt_manager::PromptManager::new(storage_of(&guard)?).save_template(&template)
}

// --- Pipeline commands ------------------------------------------------------

/// Transcribe a meeting's audio file with the configured STT provider and
/// store the resulting segments. Returns the number of segments stored.
#[tauri::command]
async fn transcribe_meeting(meeting_id: String, state: State<'_, AppState>) -> Result<usize> {
    use crate::transcription::{STTRouter, TranscriptionConfig, TranscriptionManager};

    // Gather everything we need without holding locks across awaits.
    let (meeting, provider_name, credentials) = {
        let guard = acquire_vault(&state.vault)?;
        let storage = storage_of(&guard)?;
        let config = config_of(&guard)?;
        let meeting_id = uuid::Uuid::parse_str(&meeting_id)
            .map_err(|e| RecapError::Storage(format!("Invalid meeting id: {}", e)))?;
        let meeting = storage
            .get_meeting(meeting_id)?
            .ok_or_else(|| RecapError::Storage("Meeting not found".to_string()))?;
        let provider_name = config.stt_provider()?;
        let credentials = collect_credentials(storage, config)?;
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
        let guard = acquire_vault(&state.vault)?;
        let storage = storage_of(&guard)?;
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
        let guard = acquire_vault(&state.vault)?;
        let storage = storage_of(&guard)?;
        let config = config_of(&guard)?;
        let meeting_id = uuid::Uuid::parse_str(&meeting_id)
            .map_err(|e| RecapError::Storage(format!("Invalid meeting id: {}", e)))?;
        let _meeting = storage
            .get_meeting(meeting_id)?
            .ok_or_else(|| RecapError::Storage("Meeting not found".to_string()))?;
        let provider_name = config.llm_provider()?;
        let credentials = collect_credentials(storage, config)?;
        (meeting_id, provider_name, credentials)
    };

    let segments = {
        let guard = acquire_vault(&state.vault)?;
        storage_of(&guard)?.get_transcript_segments(meeting_id)?
    };
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
    let guard = acquire_vault(&state.vault)?;
    storage_of(&guard)?.save_summary(&summary)?;

    Ok(result.summary)
}

fn main() {
    let data_dir = resolve_data_dir();
    if let Err(e) = std::fs::create_dir_all(&data_dir) {
        eprintln!("Failed to create data directory {}: {}", data_dir.display(), e);
    }

    let mut vault = VaultState {
        storage: None,
        config: None,
    };

    // Optional headless/CI auto-unlock: when RECAP_PASSPHRASE is set and a
    // vault exists, unlock automatically. Interactive users go through the
    // unlock screen instead.
    if let Ok(passphrase) = std::env::var("RECAP_PASSPHRASE") {
        if !passphrase.is_empty() && storage::vault_initialized(&data_dir) {
            match StorageManager::open(data_dir.clone(), &passphrase) {
                Ok(storage) => {
                    seed_default_templates(&storage);
                    match build_config(&data_dir) {
                        Ok(config) => {
                            vault.config = Some(config);
                            vault.storage = Some(storage);
                        }
                        Err(e) => eprintln!("Failed to open config after auto-unlock: {}", e),
                    }
                }
                Err(e) => eprintln!("RECAP_PASSPHRASE auto-unlock failed: {}", e),
            }
        }
    }

    let app_state = AppState {
        vault: Arc::new(Mutex::new(vault)),
        data_dir,
    };

    tauri::Builder::default()
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![
            vault_status,
            initialize_vault,
            unlock_vault,
            lock_vault,
            change_vault_passphrase,
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
