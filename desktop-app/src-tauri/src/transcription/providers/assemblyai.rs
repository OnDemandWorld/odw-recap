//! Scaffolded subsystem — wired up in a future milestone; see IMPROVEMENT_PLAN.md.
#![allow(dead_code)]

use crate::error::{RecapError, Result};
use crate::storage::types::TranscriptSegment;
use crate::transcription::stt_provider::{STTProvider, TranscriptionConfig, TranscriptionResult};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::path::Path;
use uuid::Uuid;

/// AssemblyAI provider
pub struct AssemblyAIProvider {
    api_key: Option<String>,
    client: reqwest::Client,
}

#[derive(Debug, Serialize)]
struct TranscriptRequest {
    audio_url: String,
    language_code: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TranscriptResponse {
    id: String,
    status: String,
    text: Option<String>,
    utterances: Option<Vec<Utterance>>,
}

#[derive(Debug, Deserialize)]
struct Utterance {
    text: String,
    start: i64,
    end: i64,
    speaker: Option<String>,
}

impl AssemblyAIProvider {
    pub fn new(api_key: Option<String>) -> Self {
        Self {
            api_key,
            client: reqwest::Client::new(),
        }
    }

    pub fn with_api_key(mut self, api_key: String) -> Self {
        self.api_key = Some(api_key);
        self
    }
}

#[async_trait]
impl STTProvider for AssemblyAIProvider {
    fn name(&self) -> &str {
        "assemblyai"
    }

    async fn is_available(&self) -> Result<bool> {
        Ok(self.api_key.is_some())
    }

    async fn transcribe(
        &self,
        _audio_path: &Path,
        config: TranscriptionConfig,
    ) -> Result<TranscriptionResult> {
        let _api_key = self.api_key.as_ref().ok_or_else(|| {
            RecapError::Provider {
                provider: "assemblyai".to_string(),
                message: "API key not configured".to_string(),
            }
        })?;

        // AssemblyAI requires audio to be accessible via URL
        // In production, you'd upload to a temporary storage or use their upload endpoint
        // For now, this is a stub that demonstrates the API structure

        let _request = TranscriptRequest {
            audio_url: "https://example.com/audio.mp3".to_string(), // Would be actual URL
            language_code: config.language.clone(),
        };

        // Stub response
        let segments = vec![TranscriptSegment {
            id: 1,
            meeting_id: Uuid::nil(),
            speaker_id: None,
            start_ms: 0,
            end_ms: 5000,
            text: "AssemblyAI transcription would appear here.".to_string(),
            confidence: 0.95,
            is_final: true,
            version: 1,
        }];

        Ok(TranscriptionResult {
            segments,
            language: config.language.unwrap_or_else(|| "en".to_string()),
            duration_seconds: 5.0,
        })
    }

    fn supported_models(&self) -> Vec<String> {
        vec!["default".to_string(), "nano".to_string()]
    }
}
