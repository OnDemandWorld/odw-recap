use crate::error::{RecapError, Result};
use crate::transcription::providers;
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

    /// Build a router with every compiled-in provider registered under the
    /// names used by the desktop UI settings screen.
    ///
    /// `credentials` maps credential keys to values; typical keys are
    /// `openai`, `deepgram`, `assemblyai`, `google_stt`, `azure_speech`,
    /// `azure_region`, `aws_access_key_id`, `aws_secret_access_key`,
    /// `aws_region`, and `whisper_model_path`. Missing entries simply leave
    /// the corresponding provider unconfigured (it reports unavailable).
    pub fn with_all_providers(default_provider: String, credentials: &HashMap<String, String>) -> Self {
        let get = |key: &str| credentials.get(key).cloned();

        let mut router = Self::new(default_provider);
        router.register_provider(
            "whisper_local".to_string(),
            Arc::new(providers::whisper_local::WhisperLocalProvider::new(get("whisper_model_path"))),
        );
        router.register_provider(
            "openai".to_string(),
            Arc::new(providers::openai_whisper::OpenAIWhisperProvider::new(get("openai"))),
        );
        router.register_provider(
            "assemblyai".to_string(),
            Arc::new(providers::assemblyai::AssemblyAIProvider::new(get("assemblyai"))),
        );
        router.register_provider(
            "deepgram".to_string(),
            Arc::new(providers::deepgram::DeepgramProvider::new(get("deepgram"))),
        );
        router.register_provider(
            "aws_transcribe".to_string(),
            Arc::new(providers::aws_transcribe::AWSTranscribeProvider::new(
                get("aws_access_key_id"),
                get("aws_secret_access_key"),
                get("aws_region").unwrap_or_else(|| "us-east-1".to_string()),
            )),
        );
        router.register_provider(
            "azure_speech".to_string(),
            Arc::new(providers::azure_speech::AzureSpeechProvider::new(
                get("azure_speech"),
                get("azure_region").unwrap_or_else(|| "eastus".to_string()),
            )),
        );
        router.register_provider(
            "google_stt".to_string(),
            Arc::new(providers::google_stt::GoogleSTTProvider::new(get("google_stt"))),
        );
        router
    }

    /// Register a provider
    pub fn register_provider(&mut self, name: String, provider: Arc<dyn STTProvider>) {
        self.providers.insert(name, provider);
    }

    /// Get a provider by name
    pub fn get_provider(&self, name: &str) -> Option<&Arc<dyn STTProvider>> {
        self.providers.get(name)
    }

    #[allow(dead_code)]
    /// Get the default provider
    pub fn get_default_provider(&self) -> Option<&Arc<dyn STTProvider>> {
        self.providers.get(&self.default_provider)
    }

    /// Transcribe using the default provider or the provider named in the config.
    pub async fn transcribe(
        &self,
        audio_path: &Path,
        config: TranscriptionConfig,
    ) -> Result<TranscriptionResult> {
        // Select by provider name. `config.model` chooses a model *within* a
        // provider and must not be used for provider lookup.
        let provider_name = config.provider.as_deref().unwrap_or(&self.default_provider);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_with_all_providers_registers_ui_names() {
        let router = STTRouter::with_all_providers("whisper_local".to_string(), &HashMap::new());
        let mut providers = router.available_providers();
        providers.sort();
        assert_eq!(
            providers,
            vec![
                "assemblyai",
                "aws_transcribe",
                "azure_speech",
                "deepgram",
                "google_stt",
                "openai",
                "whisper_local",
            ]
        );
        assert!(router.get_provider("openai").is_some());
        assert!(router.get_provider("nonexistent").is_none());
    }
}
