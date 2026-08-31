use crate::error::{RecapError, Result};
use crate::summarization::llm_provider::{LLMProvider, SummarizationConfig, SummarizationResult};
use crate::summarization::providers;
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

    /// Build a router with every compiled-in provider registered under the
    /// names used by the desktop UI settings screen.
    ///
    /// `credentials` maps credential keys to values; typical keys are
    /// `openai`, `anthropic`, `google`, `ollama_url`, `llama_model_path`,
    /// `azure_openai`, `azure_openai_endpoint`, `azure_openai_deployment`,
    /// `aws_access_key_id`, `aws_secret_access_key`, and `aws_region`.
    pub fn with_all_providers(default_provider: String, credentials: &HashMap<String, String>) -> Self {
        let get = |key: &str| credentials.get(key).cloned();

        let mut router = Self::new(default_provider);
        router.register_provider(
            "llama_local".to_string(),
            Arc::new(providers::llama_local::LlamaLocalProvider::new(get("llama_model_path"))),
        );
        router.register_provider(
            "ollama".to_string(),
            Arc::new(providers::ollama::OllamaProvider::new(get("ollama_url"))),
        );
        router.register_provider(
            "openai".to_string(),
            Arc::new(providers::openai::OpenAIProvider::new(get("openai"))),
        );
        router.register_provider(
            "anthropic".to_string(),
            Arc::new(providers::anthropic::AnthropicProvider::new(get("anthropic"))),
        );
        router.register_provider(
            "google".to_string(),
            Arc::new(providers::google::GoogleProvider::new(get("google"))),
        );
        router.register_provider(
            "aws_bedrock".to_string(),
            Arc::new(providers::aws_bedrock::AWSBedrockProvider::new(
                get("aws_access_key_id"),
                get("aws_secret_access_key"),
                get("aws_region").unwrap_or_else(|| "us-east-1".to_string()),
            )),
        );
        router.register_provider(
            "azure".to_string(),
            Arc::new(providers::azure::AzureProvider::new(
                get("azure_openai"),
                get("azure_openai_endpoint"),
                get("azure_openai_deployment"),
            )),
        );
        router
    }

    /// Register a provider
    pub fn register_provider(&mut self, name: String, provider: Arc<dyn LLMProvider>) {
        self.providers.insert(name, provider);
    }

    /// Get a provider by name
    pub fn get_provider(&self, name: &str) -> Option<&Arc<dyn LLMProvider>> {
        self.providers.get(name)
    }

    #[allow(dead_code)]
    /// Get the default provider
    pub fn get_default_provider(&self) -> Option<&Arc<dyn LLMProvider>> {
        self.providers.get(&self.default_provider)
    }

    /// Summarize using the default provider or the provider named in the config.
    pub async fn summarize(
        &self,
        text: &str,
        config: SummarizationConfig,
    ) -> Result<SummarizationResult> {
        // Select by provider name. `config.model` chooses a model *within* a
        // provider and must not be used for provider lookup.
        let provider_name = config.provider.as_deref().unwrap_or(&self.default_provider);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_with_all_providers_registers_ui_names() {
        let router = LLMRouter::with_all_providers("ollama".to_string(), &HashMap::new());
        let mut providers = router.available_providers();
        providers.sort();
        assert_eq!(
            providers,
            vec![
                "anthropic",
                "aws_bedrock",
                "azure",
                "google",
                "llama_local",
                "ollama",
                "openai",
            ]
        );
        assert!(router.get_provider("anthropic").is_some());
    }
}
