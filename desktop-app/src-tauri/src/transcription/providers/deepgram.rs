use crate::error::{RecapError, Result};
use crate::storage::types::TranscriptSegment;
use crate::transcription::stt_provider::{STTProvider, TranscriptionConfig, TranscriptionResult};
use async_trait::async_trait;
use serde::Deserialize;
use std::path::Path;
use uuid::Uuid;

/// Deepgram provider
pub struct DeepgramProvider {
    api_key: Option<String>,
    client: reqwest::Client,
}

#[derive(Debug, Deserialize)]
struct DeepgramResponse {
    results: DeepgramResults,
}

#[derive(Debug, Deserialize)]
struct DeepgramResults {
    channels: Vec<DeepgramChannel>,
}

#[derive(Debug, Deserialize)]
struct DeepgramChannel {
    alternatives: Vec<DeepgramAlternative>,
}

#[derive(Debug, Deserialize)]
struct DeepgramAlternative {
    transcript: String,
    words: Option<Vec<DeepgramWord>>,
}

#[derive(Debug, Deserialize)]
struct DeepgramWord {
    word: String,
    start: f64,
    end: f64,
    confidence: f64,
}

impl DeepgramProvider {
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
impl STTProvider for DeepgramProvider {
    fn name(&self) -> &str {
        "deepgram"
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
                provider: "deepgram".to_string(),
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

        // Make API request. The Content-Type must match the actual file
        // format, otherwise Deepgram rejects or mis-decodes the audio.
        let content_type = match audio_path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_lowercase())
            .as_deref()
        {
            Some("mp3") => "audio/mpeg",
            Some("wav") => "audio/wav",
            Some("m4a") => "audio/mp4",
            Some("ogg") => "audio/ogg",
            Some("flac") => "audio/flac",
            Some("webm") => "audio/webm",
            _ => "application/octet-stream",
        };

        let response = self
            .client
            .post("https://api.deepgram.com/v1/listen?punctuate=true")
            .header("Authorization", format!("Token {}", api_key))
            .header("Content-Type", content_type)
            .body(audio_data)
            .send()
            .await
            .map_err(|e| RecapError::Provider {
                provider: "deepgram".to_string(),
                message: e.to_string(),
            })?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(RecapError::Provider {
                provider: "deepgram".to_string(),
                message: format!("API error: {}", error_text),
            });
        }

        let dg_response: DeepgramResponse = response.json().await.map_err(|e| {
            RecapError::Provider {
                provider: "deepgram".to_string(),
                message: format!("Failed to parse response: {}", e),
            }
        })?;

        // Convert to our segment format
        let mut segment_id = 1i64;
        let segments = if let Some(channel) = dg_response.results.channels.first() {
            if let Some(alternative) = channel.alternatives.first() {
                if let Some(words) = &alternative.words {
                    // Group words into segments (simple approach: every 10 words)
                    let mut segments = Vec::new();
                    let mut current_words: Vec<String> = Vec::new();
                    let mut confidences: Vec<f64> = Vec::new();
                    let mut start_time = 0.0;

                    for (i, word) in words.iter().enumerate() {
                        if current_words.is_empty() {
                            start_time = word.start;
                        }
                        current_words.push(word.word.clone());
                        confidences.push(word.confidence);

                        if current_words.len() >= 10 || i == words.len() - 1 {
                            let end_time = word.end;
                            let text = current_words.join(" ");
                            let avg_confidence = confidences.iter().sum::<f64>()
                                / confidences.len() as f64;

                            let id = segment_id;
                            segment_id += 1;
                            segments.push(TranscriptSegment {
                                id,
                                meeting_id: Uuid::nil(),
                                speaker_id: None,
                                start_ms: (start_time * 1000.0) as i64,
                                end_ms: (end_time * 1000.0) as i64,
                                text,
                                confidence: avg_confidence as f32,
                                is_final: true,
                                version: 1,
                            });

                            current_words.clear();
                            confidences.clear();
                        }
                    }

                    segments
                } else {
                    // Single segment with full transcript
                    vec![TranscriptSegment {
                        id: 1,
                        meeting_id: Uuid::nil(),
                        speaker_id: None,
                        start_ms: 0,
                        end_ms: 0,
                        text: alternative.transcript.clone(),
                        confidence: 0.0,
                        is_final: true,
                        version: 1,
                    }]
                }
            } else {
                Vec::new()
            }
        } else {
            Vec::new()
        };

        Ok(TranscriptionResult {
            segments,
            language: config.language.unwrap_or_else(|| "en".to_string()),
            duration_seconds: 0.0,
        })
    }

    fn supported_models(&self) -> Vec<String> {
        vec!["nova".to_string(), "enhanced".to_string(), "base".to_string()]
    }
}
