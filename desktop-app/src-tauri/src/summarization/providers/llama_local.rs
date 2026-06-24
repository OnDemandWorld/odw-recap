use crate::error::{RecapError, Result};
use crate::summarization::llm_provider::{LLMProvider, SummarizationConfig, SummarizationResult};
use async_trait::async_trait;

/// Local LLM provider using llama.cpp
pub struct LlamaLocalProvider {
    model_path: Option<String>,
}

impl LlamaLocalProvider {
    pub fn new(model_path: Option<String>) -> Self {
        Self { model_path }
    }
}

#[async_trait]
impl LLMProvider for LlamaLocalProvider {
    fn name(&self) -> &str {
        "llama_local"
    }

    async fn is_available(&self) -> Result<bool> {
        Ok(self.model_path.is_some())
    }

    async fn summarize(
        &self,
        text: &str,
        config: SummarizationConfig,
    ) -> Result<SummarizationResult> {
        if !self.is_available().await? {
            return Err(RecapError::Summarization(
                "Llama model not loaded".to_string(),
            ));
        }

        // Stub implementation - would call llama.cpp FFI
        let summary = format!(
            "Local LLM summary of {} characters. This is a stub implementation.",
            text.len()
        );

        Ok(SummarizationResult {
            summary,
            action_items: Vec::new(),
            decisions: Vec::new(),
            key_points: Vec::new(),
            provider: "llama_local".to_string(),
        })
    }

    fn supported_models(&self) -> Vec<String> {
        vec![
            "llama-2-7b".to_string(),
            "llama-2-13b".to_string(),
            "llama-2-70b".to_string(),
            "codellama-7b".to_string(),
        ]
    }
}
