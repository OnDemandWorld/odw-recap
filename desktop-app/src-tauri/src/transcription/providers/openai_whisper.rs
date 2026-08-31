use crate::error::{RecapError, Result};
use crate::storage::types::TranscriptSegment;
use crate::transcription::stt_provider::{STTProvider, TranscriptionConfig, TranscriptionResult};
use async_trait::async_trait;
use serde::Deserialize;
use std::path::Path;
use uuid::Uuid;

/// OpenAI Whisper API provider
pub struct OpenAIWhisperProvider {
    api_key: Option<String>,
    client: reqwest::Client,
}

#[derive(Debug, Deserialize)]
struct WhisperResponse {
    text: String,
    segments: Option<Vec<WhisperSegment>>,
}

#[derive(Debug, Deserialize)]
struct WhisperSegment {
    start: f64,
    end: f64,
    text: String,
}

impl OpenAIWhisperProvider {
    pub fn new(api_key: Option<String>) -> Self {
        Self {
            api_key,
            client: reqwest::Client::new(),
        }
    }

    /// Builder reserved for dynamic credential injection (see IMPROVEMENT_PLAN.md).
    #[allow(dead_code)]
    pub fn with_api_key(mut self, api_key: String) -> Self {
        self.api_key = Some(api_key);
        self
    }
}

#[async_trait]
impl STTProvider for OpenAIWhisperProvider {
    fn name(&self) -> &str {
        "openai_whisper"
    }

    async fn is_available(&self) -> Result<bool> {
        Ok(self.api_key.is_some())
    }

    async fn transcribe(
        &self,
        audio_path: &Path,
        config: TranscriptionConfig,
    ) -> Result<TranscriptionResult> {
        let api_key = self.api_key.as_ref().ok_or_else(|| {
            RecapError::Provider {
                provider: "openai_whisper".to_string(),
                message: "API key not configured".to_string(),
            }
        })?;

        if !audio_path.exists() {
            return Err(RecapError::AudioInput(format!(
                "Audio file not found: {}",
                audio_path.display()
            )));
        }

        // Read audio file
        let audio_data = tokio::fs::read(audio_path).await?;

        // Create multipart form
        let part = reqwest::multipart::Part::bytes(audio_data)
            .file_name(audio_path.file_name().unwrap().to_string_lossy().to_string())
            .mime_str("audio/mpeg")
            .map_err(|e| RecapError::Transcription(e.to_string()))?;

        let form = reqwest::multipart::Form::new()
            .part("file", part)
            .text("model", config.model.unwrap_or_else(|| "whisper-1".to_string()))
            .text("response_format", "verbose_json".to_string());

        let form = if let Some(lang) = &config.language {
            form.text("language", lang.clone())
        } else {
            form
        };

        // Make API request
        let response = self
            .client
            .post("https://api.openai.com/v1/audio/transcriptions")
            .header("Authorization", format!("Bearer {}", api_key))
            .multipart(form)
            .send()
            .await
            .map_err(|e| RecapError::Provider {
                provider: "openai_whisper".to_string(),
                message: e.to_string(),
            })?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(RecapError::Provider {
                provider: "openai_whisper".to_string(),
                message: format!("API error: {}", error_text),
            });
        }

        let whisper_response: WhisperResponse = response.json().await.map_err(|e| {
            RecapError::Provider {
                provider: "openai_whisper".to_string(),
                message: format!("Failed to parse response: {}", e),
            }
        })?;

        // Convert to our segment format
        let mut segment_id = 1i64;
        let segments = if let Some(api_segments) = whisper_response.segments {
            api_segments
                .into_iter()
                .map(|seg| {
                    let id = segment_id;
                    segment_id += 1;
                    TranscriptSegment {
                        id,
                        meeting_id: Uuid::nil(),
                        speaker_id: None,
                        start_ms: (seg.start * 1000.0) as i64,
                        end_ms: (seg.end * 1000.0) as i64,
                        text: seg.text,
                        confidence: 0.0,
                        is_final: true,
                        version: 1,
                    }
                })
                .collect()
        } else {
            // Single segment with full text
            vec![TranscriptSegment {
                id: 1,
                meeting_id: Uuid::nil(),
                speaker_id: None,
                start_ms: 0,
                end_ms: 0,
                text: whisper_response.text,
                confidence: 0.0,
                is_final: true,
                version: 1,
            }]
        };

        Ok(TranscriptionResult {
            segments,
            language: config.language.unwrap_or_else(|| "en".to_string()),
            duration_seconds: 0.0, // API doesn't return duration in verbose_json
        })
    }

    fn supported_models(&self) -> Vec<String> {
        vec!["whisper-1".to_string()]
    }
}
