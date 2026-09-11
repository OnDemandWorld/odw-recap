//! Meeting processing pipeline: transcription (STT) followed by
//! summarization (LLM or rule-based), with results persisted to storage.
//!
//! Provider policy (see DEVELOPMENT.md — local engines are still stubs):
//! - STT: OpenAI Whisper API and Deepgram are real HTTP implementations.
//!   Local whisper.cpp is a stub, so it is rejected with an actionable error.
//! - Summarization: OpenAI and Anthropic are real when an API key is saved;
//!   everything else falls back to the offline rule-based summarizer.

use std::path::Path;
use std::sync::{Arc, Mutex};

use crate::error::{RecapError, Result};
use crate::storage::types::{Meeting, MeetingStatus, TranscriptSegment};
use crate::storage::StorageManager;
use crate::summarization::providers::anthropic::AnthropicProvider;
use crate::summarization::providers::openai::OpenAIProvider;
use crate::summarization::{RuleBasedSummarizer, SummarizationConfig};
use crate::transcription::providers::deepgram::DeepgramProvider;
use crate::transcription::providers::openai_whisper::OpenAIWhisperProvider;
use crate::transcription::{TranscriptionConfig, TranscriptionManager, STTRouter};
use uuid::Uuid;

/// Result of a successful `process_meeting` run.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ProcessOutcome {
    pub meeting_id: Uuid,
    pub status: String,
    pub segments: usize,
    pub duration_seconds: f64,
    pub stt_provider: String,
    pub summary_provider: String,
}

/// UI-facing provider names map to concrete STT implementations.
fn stt_provider_id(selected: &str) -> &'static str {
    match selected {
        "openai" | "openai_whisper" => "openai_whisper",
        "deepgram" => "deepgram",
        _ => "local",
    }
}

/// Transcribe `audio_path` with the cloud STT provider selected in settings.
/// Local transcription (whisper.cpp) is not implemented yet, so any other
/// selection returns an actionable error instead of stub output.
async fn transcribe(
    storage: &Arc<Mutex<StorageManager>>,
    audio_path: &Path,
    selected: &str,
    language: &str,
) -> Result<(Vec<TranscriptSegment>, String)> {
    let provider_id = stt_provider_id(selected);
    if provider_id == "local" {
        return Err(RecapError::Transcription(
            "Local transcription (whisper.cpp) is not available yet. Open Settings and \
             choose OpenAI or Deepgram as the STT provider and save its API key."
                .to_string(),
        ));
    }

    let api_key = {
        let storage = storage.lock().map_err(|_| lock_err())?;
        let provider_key_name = if provider_id == "openai_whisper" { "openai" } else { provider_id };
        storage.get_api_key(provider_key_name)?
    };
    let api_key = api_key.ok_or_else(|| {
        RecapError::Provider {
            provider: provider_id.to_string(),
            message: format!(
                "No API key saved for {}. Open Settings → API Keys, paste the key and click Save.",
                provider_id
            ),
        }
    })?;

    let mut router = STTRouter::new(provider_id.to_string());
    match provider_id {
        "openai_whisper" => router.register_provider(
            "openai_whisper".to_string(),
            Arc::new(OpenAIWhisperProvider::new(Some(api_key))),
        ),
        "deepgram" => router.register_provider(
            "deepgram".to_string(),
            Arc::new(DeepgramProvider::new(Some(api_key))),
        ),
        _ => unreachable!("local provider filtered above"),
    }
    let manager = TranscriptionManager::new(router);

    let config = TranscriptionConfig {
        language: Some(language.to_string()),
        ..TranscriptionConfig::default()
    };
    let result = manager.transcribe(audio_path, config).await?;
    Ok((result.segments, result.language))
}

/// Produce a summary for `transcript_text` using the configured LLM provider,
/// falling back to the offline rule-based summarizer whenever the selection
/// is a stub or its API key is missing.
async fn summarize(
    storage: &Arc<Mutex<StorageManager>>,
    transcript_text: &str,
    selected: &str,
) -> Result<(String, Vec<String>, Vec<String>, String)> {
    let config = SummarizationConfig {
        max_length: 500,
        ..SummarizationConfig::default()
    };

    let maybe_llm = match selected {
        "openai" => {
            let key = {
                let storage = storage.lock().map_err(|_| lock_err())?;
                storage.get_api_key("openai")?
            };
            key.map(|k| Arc::new(OpenAIProvider::new(Some(k))) as Arc<dyn crate::summarization::LLMProvider>)
        }
        "anthropic" => {
            let key = {
                let storage = storage.lock().map_err(|_| lock_err())?;
                storage.get_api_key("anthropic")?
            };
            key.map(|k| Arc::new(AnthropicProvider::new(Some(k))) as Arc<dyn crate::summarization::LLMProvider>)
        }
        _ => None,
    };

    if let Some(provider) = maybe_llm {
        match provider.summarize(transcript_text, config.clone()).await {
            Ok(result) => {
                return Ok((result.summary, result.action_items, result.decisions, result.provider));
            }
            Err(e) => {
                // Fall back to offline summarization rather than failing the
                // whole meeting — the transcript is the valuable part.
                eprintln!("LLM summarization with {} failed, falling back to rule-based: {}", selected, e);
            }
        }
    }

    let result = RuleBasedSummarizer::new().summarize(transcript_text, config)?;
    Ok((result.summary, result.action_items, result.decisions, result.provider))
}

fn lock_err() -> RecapError {
    RecapError::Storage("Failed to lock storage".to_string())
}

/// Process a meeting end to end: transcribe, summarize, persist, and update
/// its status. On any failure the meeting is marked `failed` and the error is
/// propagated so the UI can show it.
pub async fn process_meeting(
    storage: &Arc<Mutex<StorageManager>>,
    meeting_id: Uuid,
) -> Result<ProcessOutcome> {
    // Snapshot everything needed across awaits without holding the lock.
    let (audio_path, language, stt_selected, llm_selected) = {
        let storage = storage.lock().map_err(|_| lock_err())?;
        let meeting = storage
            .get_meeting(meeting_id)?
            .ok_or_else(|| RecapError::Storage(format!("Meeting not found: {}", meeting_id)))?;
        (
            meeting.audio_file_path.clone(),
            meeting.language.clone(),
            meeting.stt_provider.clone(),
            meeting.llm_provider.clone(),
        )
    };

    set_status(storage, meeting_id, MeetingStatus::Processing)?;

    let outcome = run_pipeline(storage, meeting_id, &audio_path, &language, &stt_selected, &llm_selected).await;

    match outcome {
        Ok(outcome) => {
            set_status(storage, meeting_id, MeetingStatus::Completed)?;
            Ok(outcome)
        }
        Err(e) => {
            if let Err(mark_err) = set_status(storage, meeting_id, MeetingStatus::Failed) {
                eprintln!("failed to mark meeting {} as failed: {}", meeting_id, mark_err);
            }
            Err(e)
        }
    }
}

async fn run_pipeline(
    storage: &Arc<Mutex<StorageManager>>,
    meeting_id: Uuid,
    audio_path: &Path,
    language: &str,
    stt_selected: &str,
    llm_selected: &str,
) -> Result<ProcessOutcome> {
    let (segments, _detected_language) = transcribe(storage, audio_path, stt_selected, language).await?;

    let transcript_text = segments
        .iter()
        .map(|s| s.text.trim())
        .filter(|t| !t.is_empty())
        .collect::<Vec<_>>()
        .join("\n");

    let (summary, action_items, decisions, summary_provider) =
        summarize(storage, &transcript_text, llm_selected).await?;

    // Persist everything in one critical section.
    let duration_seconds = segments
        .iter()
        .map(|s| s.end_ms.max(s.start_ms))
        .max()
        .unwrap_or(0) as f64
        / 1000.0;
    {
        let storage = storage.lock().map_err(|_| lock_err())?;
        storage.save_transcript_segments(meeting_id, &segments)?;
        storage.save_summary(meeting_id, &summary, &summary_provider, Some(&summary_provider))?;
        storage.save_action_items(meeting_id, &action_items)?;
        storage.save_decisions(meeting_id, &decisions)?;

        if let Some(mut meeting) = storage.get_meeting(meeting_id)? {
            meeting.duration_seconds = Some(duration_seconds.round() as i32);
            meeting.model_used = summary_provider.clone();
            meeting.updated_at = chrono::Utc::now().timestamp_millis();
            storage.update_meeting(&meeting)?;
        }
    }

    Ok(ProcessOutcome {
        meeting_id,
        status: MeetingStatus::Completed.to_string(),
        segments: segments.len(),
        duration_seconds,
        stt_provider: stt_provider_id(stt_selected).to_string(),
        summary_provider,
    })
}

fn set_status(
    storage: &Arc<Mutex<StorageManager>>,
    meeting_id: Uuid,
    status: MeetingStatus,
) -> Result<()> {
    let storage = storage.lock().map_err(|_| lock_err())?;
    let mut meeting: Meeting = storage
        .get_meeting(meeting_id)?
        .ok_or_else(|| RecapError::Storage(format!("Meeting not found: {}", meeting_id)))?;
    meeting.status = status;
    meeting.updated_at = chrono::Utc::now().timestamp_millis();
    storage.update_meeting(&meeting)
}
