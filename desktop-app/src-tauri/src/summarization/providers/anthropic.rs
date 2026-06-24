use crate::error::{RecapError, Result};
use crate::summarization::llm_provider::{LLMProvider, SummarizationConfig, SummarizationResult};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// Anthropic Claude LLM provider
pub struct AnthropicProvider {
    api_key: Option<String>,
    client: reqwest::Client,
}

#[derive(Debug, Serialize)]
struct AnthropicRequest {
    model: String,
    messages: Vec<AnthropicMessage>,
    max_tokens: usize,
}

#[derive(Debug, Serialize)]
struct AnthropicMessage {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct AnthropicResponse {
    content: Vec<AnthropicContent>,
}

#[derive(Debug, Deserialize)]
struct AnthropicContent {
    text: String,
}

impl AnthropicProvider {
    pub fn new(api_key: Option<String>) -> Self {
        Self {
            api_key,
            client: reqwest::Client::new(),
        }
    }

    pub fn with_api_key(mut self, api_key: String) -> Self {
        self.api_key = Some(api_key);
        self
    }
}

#[async_trait]
impl LLMProvider for AnthropicProvider {
    fn name(&self) -> &str {
        "anthropic"
    }

    async fn is_available(&self) -> Result<bool> {
        Ok(self.api_key.is_some())
    }

    async fn summarize(
        &self,
        text: &str,
        config: SummarizationConfig,
    ) -> Result<SummarizationResult> {
        let api_key = self.api_key.as_ref().ok_or_else(|| {
            RecapError::Provider {
                provider: "anthropic".to_string(),
                message: "API key not configured".to_string(),
            }
        })?;

        let model = config.model.unwrap_or_else(|| "claude-3-sonnet-20240229".to_string());

        let prompt = format!(
            "Summarize the following text in {} words or less:\n\n{}",
            config.max_length / 5,
            text
        );

        let request = AnthropicRequest {
            model,
            messages: vec![AnthropicMessage {
                role: "user".to_string(),
                content: prompt,
            }],
            max_tokens: config.max_length,
        };

        let response = self
            .client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", api_key)
            .header("anthropic-version", "2023-06-01")
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| RecapError::Provider {
                provider: "anthropic".to_string(),
                message: e.to_string(),
            })?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(RecapError::Provider {
                provider: "anthropic".to_string(),
                message: format!("API error: {}", error_text),
            });
        }

        let anthropic_response: AnthropicResponse = response.json().await.map_err(|e| {
            RecapError::Provider {
                provider: "anthropic".to_string(),
                message: format!("Failed to parse response: {}", e),
            }
        })?;

        let summary = anthropic_response
            .content
            .first()
            .map(|c| c.text.clone())
            .unwrap_or_default();

        Ok(SummarizationResult {
            summary,
            action_items: Vec::new(),
            decisions: Vec::new(),
            key_points: Vec::new(),
            provider: "anthropic".to_string(),
        })
    }

    fn supported_models(&self) -> Vec<String> {
        vec![
            "claude-3-opus-20240229".to_string(),
            "claude-3-sonnet-20240229".to_string(),
            "claude-3-haiku-20240307".to_string(),
            "claude-2.1".to_string(),
        ]
    }
}
