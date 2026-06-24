use crate::error::{RecapError, Result};
use crate::summarization::llm_provider::{LLMProvider, SummarizationConfig, SummarizationResult};
use async_trait::async_trait;

/// Azure OpenAI LLM provider
pub struct AzureProvider {
    api_key: Option<String>,
    endpoint: Option<String>,
    deployment_name: Option<String>,
    client: reqwest::Client,
}

impl AzureProvider {
    pub fn new(
        api_key: Option<String>,
        endpoint: Option<String>,
        deployment_name: Option<String>,
    ) -> Self {
        Self {
            api_key,
            endpoint,
            deployment_name,
            client: reqwest::Client::new(),
        }
    }

    pub fn with_credentials(
        mut self,
        api_key: String,
        endpoint: String,
        deployment_name: String,
    ) -> Self {
        self.api_key = Some(api_key);
        self.endpoint = Some(endpoint);
        self.deployment_name = Some(deployment_name);
        self
    }
}

#[async_trait]
impl LLMProvider for AzureProvider {
    fn name(&self) -> &str {
        "azure"
    }

    async fn is_available(&self) -> Result<bool> {
        Ok(self.api_key.is_some() && self.endpoint.is_some())
    }

    async fn summarize(
        &self,
        text: &str,
        config: SummarizationConfig,
    ) -> Result<SummarizationResult> {
        let _api_key = self.api_key.as_ref().ok_or_else(|| {
            RecapError::Provider {
                provider: "azure".to_string(),
                message: "Azure credentials not configured".to_string(),
            }
        })?;

        // Stub implementation
        let summary = format!(
            "Azure OpenAI summary of {} characters. This is a stub implementation.",
            text.len()
        );

        Ok(SummarizationResult {
            summary,
            action_items: Vec::new(),
            decisions: Vec::new(),
            key_points: Vec::new(),
            provider: "azure".to_string(),
        })
    }

    fn supported_models(&self) -> Vec<String> {
        vec![
            "gpt-35-turbo".to_string(),
            "gpt-4".to_string(),
            "gpt-4-32k".to_string(),
        ]
    }
}
