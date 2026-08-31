//! Scaffolded subsystem — wired up in a future milestone; see IMPROVEMENT_PLAN.md.
#![allow(dead_code)]

use crate::error::{RecapError, Result};
use crate::summarization::llm_provider::{LLMProvider, SummarizationConfig, SummarizationResult};
use async_trait::async_trait;

/// Google Gemini LLM provider
pub struct GoogleProvider {
    api_key: Option<String>,
    client: reqwest::Client,
}

impl GoogleProvider {
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
impl LLMProvider for GoogleProvider {
    fn name(&self) -> &str {
        "google"
    }

    async fn is_available(&self) -> Result<bool> {
        Ok(self.api_key.is_some())
    }

    async fn summarize(
        &self,
        text: &str,
        _config: SummarizationConfig,
    ) -> Result<SummarizationResult> {
        let _api_key = self.api_key.as_ref().ok_or_else(|| {
            RecapError::Provider {
                provider: "google".to_string(),
                message: "API key not configured".to_string(),
            }
        })?;

        // Stub implementation
        let summary = format!(
            "Google Gemini summary of {} characters. This is a stub implementation.",
            text.len()
        );

        Ok(SummarizationResult {
            summary,
            action_items: Vec::new(),
            decisions: Vec::new(),
            key_points: Vec::new(),
            provider: "google".to_string(),
        })
    }

    fn supported_models(&self) -> Vec<String> {
        vec![
            "gemini-pro".to_string(),
            "gemini-pro-vision".to_string(),
        ]
    }
}
