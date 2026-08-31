use crate::error::{RecapError, Result};
use crate::storage::types::TranscriptSegment;
use crate::transcription::stt_provider::{STTProvider, TranscriptionConfig, TranscriptionResult};
use async_trait::async_trait;
use std::path::Path;
use uuid::Uuid;

/// Local Whisper provider using whisper.cpp
pub struct WhisperLocalProvider {
    model_path: Option<String>,
}

impl WhisperLocalProvider {
    pub fn new(model_path: Option<String>) -> Self {
        Self { model_path }
    }
    // A real implementation would add model loading here via whisper.cpp FFI;
    // see IMPROVEMENT_PLAN.md.
}

#[async_trait]
impl STTProvider for WhisperLocalProvider {
    fn name(&self) -> &str {
        "whisper_local"
    }

    async fn is_available(&self) -> Result<bool> {
        // Check if model is loaded
        Ok(self.model_path.is_some())
    }

    async fn transcribe(
        &self,
        audio_path: &Path,
        config: TranscriptionConfig,
    ) -> Result<TranscriptionResult> {
        if !self.is_available().await? {
            return Err(RecapError::Transcription(
                "Whisper model not loaded".to_string(),
            ));
        }

        // Verify audio file exists
        if !audio_path.exists() {
            return Err(RecapError::AudioInput(format!(
                "Audio file not found: {}",
                audio_path.display()
            )));
        }

        // Stub implementation - in reality, this would:
        // 1. Convert audio to 16kHz mono WAV if needed
        // 2. Call whisper.cpp FFI to transcribe
        // 3. Parse output into segments with timestamps
        // 4. Optionally run speaker diarization

        // Simulate transcription with stub data
        let segments = vec![
            TranscriptSegment {
                id: 1,
                meeting_id: Uuid::nil(), // Would be set by caller
                speaker_id: None,
                start_ms: 0,
                end_ms: 3000,
                text: "This is a stub transcription segment.".to_string(),
                confidence: 0.95,
                is_final: true,
                version: 1,
            },
            TranscriptSegment {
                id: 2,
                meeting_id: Uuid::nil(),
                speaker_id: None,
                start_ms: 3000,
                end_ms: 6000,
                text: "The actual implementation would use whisper.cpp.".to_string(),
                confidence: 0.92,
                is_final: true,
                version: 1,
            },
        ];

        Ok(TranscriptionResult {
            segments,
            language: config.language.unwrap_or_else(|| "en".to_string()),
            duration_seconds: 6.0,
        })
    }

    fn supported_models(&self) -> Vec<String> {
        vec![
            "tiny".to_string(),
            "base".to_string(),
            "small".to_string(),
            "medium".to_string(),
            "large".to_string(),
            "large-v2".to_string(),
        ]
    }
}
