//! Scaffolded subsystem — wired up in a future milestone; see IMPROVEMENT_PLAN.md.
#![allow(dead_code)]

use crate::error::{RecapError, Result};
use crate::storage::types::TranscriptSegment;
use crate::transcription::stt_provider::{STTProvider, TranscriptionConfig, TranscriptionResult};
use async_trait::async_trait;
use std::path::Path;
use uuid::Uuid;

/// Google Speech-to-Text provider
pub struct GoogleSTTProvider {
    api_key: Option<String>,
    client: reqwest::Client,
}

impl GoogleSTTProvider {
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
impl STTProvider for GoogleSTTProvider {
    fn name(&self) -> &str {
        "google_stt"
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
                provider: "google_stt".to_string(),
                message: "Google API key not configured".to_string(),
            }
        })?;

        // Google Speech-to-Text integration would require:
        // 1. Convert audio to base64 or use GCS URI
        // 2. POST to speech:recognize endpoint
        // 3. Parse response with recognized text

        // Stub implementation
        let segments = vec![TranscriptSegment {
            id: 1,
            meeting_id: Uuid::nil(),
            speaker_id: None,
            start_ms: 0,
            end_ms: 5000,
            text: "Google Speech-to-Text transcription would appear here.".to_string(),
            confidence: 0.95,
            is_final: true,
            version: 1,
        }];

        Ok(TranscriptionResult {
            segments,
            language: config.language.unwrap_or_else(|| "en-US".to_string()),
            duration_seconds: 5.0,
        })
    }

    fn supported_models(&self) -> Vec<String> {
        vec!["default".to_string(), "latest_long".to_string(), "latest_short".to_string()]
    }
}
