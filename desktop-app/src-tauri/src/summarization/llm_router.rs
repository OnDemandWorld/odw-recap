use crate::error::{RecapError, Result};
use crate::summarization::llm_provider::{LLMProvider, SummarizationConfig, SummarizationResult};
use std::collections::HashMap;
use std::sync::Arc;

/// Router that selects and invokes the appropriate LLM provider
pub struct LLMRouter {
    providers: HashMap<String, Arc<dyn LLMProvider>>,
    default_provider: String,
}

impl LLMRouter {
    pub fn new(default_provider: String) -> Self {
        Self {
            providers: HashMap::new(),
            default_provider,
        }
    }

    /// Register a provider
    pub fn register_provider(&mut self, name: String, provider: Arc<dyn LLMProvider>) {
        self.providers.insert(name, provider);
    }

    /// Get a provider by name
    pub fn get_provider(&self, name: &str) -> Option<&Arc<dyn LLMProvider>> {
        self.providers.get(name)
    }

    /// Get the default provider
    pub fn get_default_provider(&self) -> Option<&Arc<dyn LLMProvider>> {
        self.providers.get(&self.default_provider)
    }

    /// Summarize using the default provider or a specified provider
    pub async fn summarize(
        &self,
        text: &str,
        config: SummarizationConfig,
    ) -> Result<SummarizationResult> {
        let provider_name = config.model.as_ref().unwrap_or(&self.default_provider);
        let provider = self
            .providers
            .get(provider_name)
            .ok_or_else(|| RecapError::Summarization(format!("Provider not found: {}", provider_name)))?;

        provider.summarize(text, config).await
    }

    /// Get list of available providers
    pub fn available_providers(&self) -> Vec<String> {
        self.providers.keys().cloned().collect()
    }
}
