pub mod audio_decode;
pub mod providers;
pub mod stt_provider;
pub mod stt_router;
pub mod whisper_models;

use crate::error::Result;
use std::path::Path;

pub use stt_provider::{STTProvider, TranscriptionConfig, TranscriptionResult};
pub use stt_router::STTRouter;

/// Transcription manager that coordinates transcription across providers
pub struct TranscriptionManager {
    router: STTRouter,
}

impl TranscriptionManager {
    pub fn new(router: STTRouter) -> Self {
        Self { router }
    }

    /// Transcribe an audio file using the configured provider
    pub async fn transcribe(
        &self,
        audio_path: &Path,
        config: TranscriptionConfig,
    ) -> Result<TranscriptionResult> {
        self.router.transcribe(audio_path, config).await
    }

    #[allow(dead_code)]
    /// Get list of available STT providers
    pub fn available_providers(&self) -> Vec<String> {
        self.router.available_providers()
    }
}
