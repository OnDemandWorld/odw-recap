use crate::error::{RecapError, Result};
use crate::summarization::llm_provider::{LLMProvider, SummarizationConfig, SummarizationResult};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// Ollama local LLM provider
pub struct OllamaProvider {
    base_url: String,
    client: reqwest::Client,
}

#[derive(Debug, Serialize)]
struct OllamaRequest {
    model: String,
    prompt: String,
    stream: bool,
}

#[derive(Debug, Deserialize)]
struct OllamaResponse {
    response: String,
}

impl OllamaProvider {
    pub fn new(base_url: Option<String>) -> Self {
        Self {
            base_url: base_url.unwrap_or_else(|| "http://localhost:11434".to_string()),
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl LLMProvider for OllamaProvider {
    fn name(&self) -> &str {
        "ollama"
    }

    async fn is_available(&self) -> Result<bool> {
        // Check if Ollama server is running
        let response = self
            .client
            .get(format!("{}/api/tags", self.base_url))
            .send()
            .await;

        Ok(response.is_ok())
    }

    async fn summarize(
        &self,
        text: &str,
        config: SummarizationConfig,
    ) -> Result<SummarizationResult> {
        let model = config.model.unwrap_or_else(|| "llama2".to_string());

        let prompt = format!(
            "Summarize the following text in {} words or less:\n\n{}",
            config.max_length / 5,
            text
        );

        let request = OllamaRequest {
            model: model.clone(),
            prompt,
            stream: false,
        };

        let response = self
            .client
            .post(format!("{}/api/generate", self.base_url))
            .json(&request)
            .send()
            .await
            .map_err(|e| RecapError::Provider {
                provider: "ollama".to_string(),
                message: e.to_string(),
            })?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(RecapError::Provider {
                provider: "ollama".to_string(),
                message: format!("API error: {}", error_text),
            });
        }

        let ollama_response: OllamaResponse = response.json().await.map_err(|e| {
            RecapError::Provider {
                provider: "ollama".to_string(),
                message: format!("Failed to parse response: {}", e),
            }
        })?;

        Ok(SummarizationResult {
            summary: ollama_response.response,
            action_items: Vec::new(),
            decisions: Vec::new(),
            key_points: Vec::new(),
            provider: "ollama".to_string(),
        })
    }

    fn supported_models(&self) -> Vec<String> {
        vec![
            "llama2".to_string(),
            "llama2:13b".to_string(),
            "mistral".to_string(),
            "mixtral".to_string(),
            "neural-chat".to_string(),
        ]
    }
}
