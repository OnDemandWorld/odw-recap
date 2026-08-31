use crate::error::{RecapError, Result};
use crate::summarization::llm_provider::{LLMProvider, SummarizationConfig, SummarizationResult};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// OpenAI LLM provider
pub struct OpenAIProvider {
    api_key: Option<String>,
    client: reqwest::Client,
}

#[derive(Debug, Serialize)]
struct OpenAIRequest {
    model: String,
    messages: Vec<OpenAIMessage>,
    max_tokens: usize,
}

#[derive(Debug, Serialize, Deserialize)]
struct OpenAIMessage {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct OpenAIResponse {
    choices: Vec<OpenAIChoice>,
}

#[derive(Debug, Deserialize)]
struct OpenAIChoice {
    message: OpenAIMessage,
}

impl OpenAIProvider {
    pub fn new(api_key: Option<String>) -> Self {
        Self {
            api_key,
            client: reqwest::Client::new(),
        }
    }

    /// Builder reserved for dynamic credential injection (see IMPROVEMENT_PLAN.md).
    #[allow(dead_code)]
    pub fn with_api_key(mut self, api_key: String) -> Self {
        self.api_key = Some(api_key);
        self
    }
}

#[async_trait]
impl LLMProvider for OpenAIProvider {
    fn name(&self) -> &str {
        "openai"
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
                provider: "openai".to_string(),
                message: "API key not configured".to_string(),
            }
        })?;

        let model = config.model.unwrap_or_else(|| "gpt-3.5-turbo".to_string());

        let prompt = format!(
            "Summarize the following text in {} words or less. {}\n\n{}",
            config.max_length / 5,
            if config.include_action_items {
                "Extract action items."
            } else {
                ""
            },
            text
        );

        let request = OpenAIRequest {
            model,
            messages: vec![OpenAIMessage {
                role: "user".to_string(),
                content: prompt,
            }],
            max_tokens: config.max_length,
        };

        let response = self
            .client
            .post("https://api.openai.com/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| RecapError::Provider {
                provider: "openai".to_string(),
                message: e.to_string(),
            })?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(RecapError::Provider {
                provider: "openai".to_string(),
                message: format!("API error: {}", error_text),
            });
        }

        let openai_response: OpenAIResponse = response.json().await.map_err(|e| {
            RecapError::Provider {
                provider: "openai".to_string(),
                message: format!("Failed to parse response: {}", e),
            }
        })?;

        let summary = openai_response
            .choices
            .first()
            .map(|c| c.message.content.clone())
            .unwrap_or_default();

        Ok(SummarizationResult {
            summary,
            action_items: Vec::new(),
            decisions: Vec::new(),
            key_points: Vec::new(),
            provider: "openai".to_string(),
        })
    }

    fn supported_models(&self) -> Vec<String> {
        vec![
            "gpt-3.5-turbo".to_string(),
            "gpt-4".to_string(),
            "gpt-4-turbo".to_string(),
        ]
    }
}
