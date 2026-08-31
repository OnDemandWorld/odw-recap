//! Scaffolded subsystem — wired up in a future milestone; see IMPROVEMENT_PLAN.md.
#![allow(dead_code)]

use crate::error::{RecapError, Result};
use crate::summarization::llm_provider::{LLMProvider, SummarizationConfig, SummarizationResult};
use async_trait::async_trait;

/// AWS Bedrock LLM provider
pub struct AWSBedrockProvider {
    access_key_id: Option<String>,
    secret_access_key: Option<String>,
    region: String,
    client: reqwest::Client,
}

impl AWSBedrockProvider {
    pub fn new(
        access_key_id: Option<String>,
        secret_access_key: Option<String>,
        region: String,
    ) -> Self {
        Self {
            access_key_id,
            secret_access_key,
            region,
            client: reqwest::Client::new(),
        }
    }

    pub fn with_credentials(
        mut self,
        access_key_id: String,
        secret_access_key: String,
    ) -> Self {
        self.access_key_id = Some(access_key_id);
        self.secret_access_key = Some(secret_access_key);
        self
    }
}

#[async_trait]
impl LLMProvider for AWSBedrockProvider {
    fn name(&self) -> &str {
        "aws_bedrock"
    }

    async fn is_available(&self) -> Result<bool> {
        Ok(self.access_key_id.is_some() && self.secret_access_key.is_some())
    }

    async fn summarize(
        &self,
        text: &str,
        _config: SummarizationConfig,
    ) -> Result<SummarizationResult> {
        let _access_key = self.access_key_id.as_ref().ok_or_else(|| {
            RecapError::Provider {
                provider: "aws_bedrock".to_string(),
                message: "AWS credentials not configured".to_string(),
            }
        })?;

        // Stub implementation
        let summary = format!(
            "AWS Bedrock summary of {} characters. This is a stub implementation.",
            text.len()
        );

        Ok(SummarizationResult {
            summary,
            action_items: Vec::new(),
            decisions: Vec::new(),
            key_points: Vec::new(),
            provider: "aws_bedrock".to_string(),
        })
    }

    fn supported_models(&self) -> Vec<String> {
        vec![
            "anthropic.claude-3-sonnet-20240229-v1:0".to_string(),
            "anthropic.claude-3-haiku-20240307-v1:0".to_string(),
            "meta.llama2-13b-chat-v1".to_string(),
            "meta.llama2-70b-chat-v1".to_string(),
        ]
    }
}
