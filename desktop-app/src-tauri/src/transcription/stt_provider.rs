use crate::error::Result;
use crate::storage::types::TranscriptSegment;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Configuration for transcription
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptionConfig {
    pub language: Option<String>,
    pub model: Option<String>,
    pub detect_speakers: bool,
    pub timestamps: bool,
    pub api_key: Option<String>,
}

impl Default for TranscriptionConfig {
    fn default() -> Self {
        Self {
            language: Some("en".to_string()),
            model: None,
            detect_speakers: false,
            timestamps: true,
            api_key: None,
        }
    }
}

/// Result of transcription
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptionResult {
    pub segments: Vec<TranscriptSegment>,
    pub language: String,
    pub duration_seconds: f64,
}

/// Trait for STT providers
#[async_trait]
pub trait STTProvider: Send + Sync {
    /// Get provider name
    fn name(&self) -> &str;

    /// Check if provider is available (e.g., API key configured, model downloaded)
    async fn is_available(&self) -> Result<bool>;

    /// Transcribe an audio file
    async fn transcribe(
        &self,
        audio_path: &Path,
        config: TranscriptionConfig,
    ) -> Result<TranscriptionResult>;

    /// Get list of supported models
    fn supported_models(&self) -> Vec<String>;
}
