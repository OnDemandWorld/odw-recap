use crate::error::{RecapError, Result};
use crate::transcription::stt_provider::{STTProvider, TranscriptionConfig, TranscriptionResult};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

/// Router that selects and invokes the appropriate STT provider
pub struct STTRouter {
    providers: HashMap<String, Arc<dyn STTProvider>>,
    default_provider: String,
}

impl STTRouter {
    pub fn new(default_provider: String) -> Self {
        Self {
            providers: HashMap::new(),
            default_provider,
        }
    }

    /// Register a provider
    pub fn register_provider(&mut self, name: String, provider: Arc<dyn STTProvider>) {
        self.providers.insert(name, provider);
    }

    /// Get a provider by name
    pub fn get_provider(&self, name: &str) -> Option<&Arc<dyn STTProvider>> {
        self.providers.get(name)
    }

    /// Get the default provider
    pub fn get_default_provider(&self) -> Option<&Arc<dyn STTProvider>> {
        self.providers.get(&self.default_provider)
    }

    /// Transcribe using the default provider or a specified provider
    pub async fn transcribe(
        &self,
        audio_path: &Path,
        config: TranscriptionConfig,
    ) -> Result<TranscriptionResult> {
        let provider_name = config.model.as_ref().unwrap_or(&self.default_provider);
        let provider = self
            .providers
            .get(provider_name)
            .ok_or_else(|| RecapError::Transcription(format!("Provider not found: {}", provider_name)))?;

        provider.transcribe(audio_path, config).await
    }

    /// Get list of available providers
    pub fn available_providers(&self) -> Vec<String> {
        self.providers.keys().cloned().collect()
    }
}
