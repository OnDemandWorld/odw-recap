pub mod llm_provider;
pub mod llm_router;
pub mod providers;
pub mod rule_based;

use crate::error::Result;
use crate::storage::types::Summary;
use std::path::Path;

pub use llm_provider::{LLMProvider, SummarizationConfig, SummarizationResult};
pub use llm_router::LLMRouter;

/// Summarization manager that coordinates summarization across providers
pub struct SummarizationManager {
    router: LLMRouter,
    rule_based: rule_based::RuleBasedSummarizer,
}

impl SummarizationManager {
    pub fn new(router: LLMRouter) -> Self {
        Self {
            router,
            rule_based: rule_based::RuleBasedSummarizer::new(),
        }
    }

    /// Summarize text using the configured provider
    pub async fn summarize(
        &self,
        text: &str,
        config: SummarizationConfig,
    ) -> Result<SummarizationResult> {
        // Use rule-based if no LLM provider specified
        if config.use_rule_based {
            return Ok(self.rule_based.summarize(text, config.clone())?);
        }

        self.router.summarize(text, config).await
    }

    /// Get list of available LLM providers
    pub fn available_providers(&self) -> Vec<String> {
        self.router.available_providers()
    }
}
