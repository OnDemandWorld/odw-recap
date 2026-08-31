use crate::error::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// Configuration for summarization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SummarizationConfig {
    /// Which registered provider to use; falls back to the router default.
    pub provider: Option<String>,
    pub max_length: usize,
    pub include_action_items: bool,
    pub include_decisions: bool,
    pub include_key_points: bool,
    pub prompt_template: Option<String>,
    pub model: Option<String>,
    pub api_key: Option<String>,
    pub use_rule_based: bool,
}

impl Default for SummarizationConfig {
    fn default() -> Self {
        Self {
            provider: None,
            max_length: 500,
            include_action_items: true,
            include_decisions: true,
            include_key_points: true,
            prompt_template: None,
            model: None,
            api_key: None,
            use_rule_based: false,
        }
    }
}

/// Result of summarization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SummarizationResult {
    pub summary: String,
    pub action_items: Vec<String>,
    pub decisions: Vec<String>,
    pub key_points: Vec<String>,
    pub provider: String,
}

/// Trait for LLM providers
#[async_trait]
pub trait LLMProvider: Send + Sync {
    /// Get provider name
    fn name(&self) -> &str;

    /// Check if provider is available (e.g., API key configured, model downloaded)
    async fn is_available(&self) -> Result<bool>;

    /// Generate a summary from text
    async fn summarize(
        &self,
        text: &str,
        config: SummarizationConfig,
    ) -> Result<SummarizationResult>;

    /// Get list of supported models
    fn supported_models(&self) -> Vec<String>;
}
