use crate::error::{RecapError, Result};
use crate::storage::types::TranscriptSegment;
use crate::transcription::stt_provider::{STTProvider, TranscriptionConfig, TranscriptionResult};
use async_trait::async_trait;
use std::path::Path;
use uuid::Uuid;

/// Azure Speech Services provider
pub struct AzureSpeechProvider {
    subscription_key: Option<String>,
    region: String,
    client: reqwest::Client,
}

impl AzureSpeechProvider {
    pub fn new(subscription_key: Option<String>, region: String) -> Self {
        Self {
            subscription_key,
            region,
            client: reqwest::Client::new(),
        }
    }

    pub fn with_subscription_key(mut self, subscription_key: String) -> Self {
        self.subscription_key = Some(subscription_key);
        self
    }
}

#[async_trait]
impl STTProvider for AzureSpeechProvider {
    fn name(&self) -> &str {
        "azure_speech"
    }

    async fn is_available(&self) -> Result<bool> {
        Ok(self.subscription_key.is_some())
    }

    async fn transcribe(
        &self,
        audio_path: &Path,
        config: TranscriptionConfig,
    ) -> Result<TranscriptionResult> {
        let _subscription_key = self.subscription_key.as_ref().ok_or_else(|| {
            RecapError::Provider {
                provider: "azure_speech".to_string(),
                message: "Azure subscription key not configured".to_string(),
            }
        })?;

        // Azure Speech integration would require:
        // 1. REST API call to speech-to-text endpoint
        // 2. Include audio file in request body
        // 3. Parse response with recognized text

        // Stub implementation
        let segments = vec![TranscriptSegment {
            id: 1,
            meeting_id: Uuid::nil(),
            speaker_id: None,
            start_ms: 0,
            end_ms: 5000,
            text: "Azure Speech transcription would appear here.".to_string(),
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
        vec!["default".to_string()]
    }
}
