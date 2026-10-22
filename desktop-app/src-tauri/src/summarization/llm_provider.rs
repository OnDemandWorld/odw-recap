use crate::error::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// Configuration for summarization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SummarizationConfig {
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

/// Prompt suffix requesting machine-parseable structured output (R-3): every
/// cloud LLM provider previously returned free prose with action_items/decisions
/// hard-coded to empty — the "No action items detected" bug that silently
/// severed the Recap→Loop webhook chain.
pub const STRUCTURED_OUTPUT_INSTRUCTION: &str = r#"

Respond with ONLY a JSON object (no markdown fences, no prose) in exactly this shape:
{"summary": "<one-paragraph summary>", "action_items": ["<actionable item>", ...], "decisions": ["<decision made>", ...], "key_points": ["<key point>", ...]}
Use empty arrays when a category has nothing."#;

/// Parse a provider reply into a `SummarizationResult`.
///
/// Accepts the JSON object either bare or fenced (```json …```), tolerates
/// non-string array entries, and falls back to: whole text as the summary plus
/// the RULE-BASED extractor's action items/decisions — so a chatty model can
/// never zero out the action-item chain again.
pub fn parse_structured_summary(raw: &str, provider_name: &str) -> SummarizationResult {
    let fallback = || {
        let rule = crate::summarization::rule_based::RuleBasedSummarizer::new();
        let config = SummarizationConfig::default();
        match rule.summarize(raw, config) {
            Ok(result) => result,
            Err(_) => SummarizationResult {
                summary: raw.trim().to_string(),
                action_items: Vec::new(),
                decisions: Vec::new(),
                key_points: Vec::new(),
                provider: provider_name.to_string(),
            },
        }
    };

    let trimmed = raw.trim();
    // Strip optional markdown fences.
    let body = if let Some(start) = trimmed.find("```") {
        let after = &trimmed[start + 3..];
        let after = after.strip_prefix("json").unwrap_or(after);
        match after.rfind("```") {
            Some(end) => &after[..end],
            None => after,
        }
    } else {
        trimmed
    };
    // Fall back to the first '{' … last '}' window when the model added prose.
    let candidate = if body.starts_with('{') {
        body
    } else if let (Some(start), Some(end)) = (body.find('{'), body.rfind('}')) {
        if end > start {
            &body[start..=end]
        } else {
            return fallback();
        }
    } else {
        return fallback();
    };

    match serde_json::from_str::<serde_json::Value>(candidate) {
        Ok(value) => {
            let strings = |key: &str| -> Vec<String> {
                value
                    .get(key)
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|item| item.as_str().map(str::to_string))
                            .collect()
                    })
                    .unwrap_or_default()
            };
            let summary = value
                .get("summary")
                .and_then(|v| v.as_str())
                .map(str::to_string)
                .unwrap_or_else(|| trimmed.to_string());
            SummarizationResult {
                summary,
                action_items: strings("action_items"),
                decisions: strings("decisions"),
                key_points: strings("key_points"),
                provider: provider_name.to_string(),
            }
        }
        Err(_) => fallback(),
    }
}

