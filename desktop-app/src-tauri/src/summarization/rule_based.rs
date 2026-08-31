use crate::error::Result;
use crate::summarization::llm_provider::{SummarizationConfig, SummarizationResult};
use std::collections::HashSet;

/// Rule-based summarizer using extractive methods
pub struct RuleBasedSummarizer;

impl RuleBasedSummarizer {
    pub fn new() -> Self {
        Self
    }

    /// Summarize text using rule-based extractive methods
    pub fn summarize(&self, text: &str, config: SummarizationConfig) -> Result<SummarizationResult> {
        let sentences = self.split_sentences(text);
        let word_freq = self.calculate_word_frequency(&sentences);
        let sentence_scores = self.score_sentences(&sentences, &word_freq);

        // Select top sentences
        let mut scored_sentences: Vec<(usize, f32)> = sentence_scores
            .into_iter()
            .enumerate()
            .collect();
        scored_sentences.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

        let num_sentences = (config.max_length / 20).max(1).min(sentences.len());
        let mut selected_indices: Vec<usize> = scored_sentences
            .iter()
            .take(num_sentences)
            .map(|(i, _)| *i)
            .collect();
        selected_indices.sort();

        let summary = selected_indices
            .iter()
            .map(|&i| sentences[i].trim())
            .collect::<Vec<_>>()
            .join(" ");

        // Extract action items and decisions (simple keyword matching)
        let action_items = self.extract_action_items(text);
        let decisions = self.extract_decisions(text);
        let key_points = self.extract_key_points(&sentences, &selected_indices);

        Ok(SummarizationResult {
            summary,
            action_items,
            decisions,
            key_points,
            provider: "rule_based".to_string(),
        })
    }

    fn split_sentences<'a>(&self, text: &'a str) -> Vec<&'a str> {
        text.split(|c| c == '.' || c == '!' || c == '?')
            .filter(|s| !s.trim().is_empty())
            .collect()
    }

    fn calculate_word_frequency(&self, sentences: &[&str]) -> std::collections::HashMap<String, f32> {
        let mut freq: std::collections::HashMap<String, f32> = std::collections::HashMap::new();
        let stop_words: HashSet<&str> = [
            "the", "a", "an", "and", "or", "but", "in", "on", "at", "to", "for", "of", "with",
            "by", "is", "are", "was", "were", "be", "been", "being", "have", "has", "had", "do",
            "does", "did", "will", "would", "could", "should", "may", "might", "can", "this",
            "that", "these", "those", "i", "you", "he", "she", "it", "we", "they",
        ]
        .iter()
        .cloned()
        .collect();

        for sentence in sentences {
            for word in sentence.split_whitespace() {
                let word = word.to_lowercase();
                let word = word.trim_matches(|c: char| !c.is_alphanumeric());
                if !word.is_empty() && !stop_words.contains(word) {
                    *freq.entry(word.to_string()).or_insert(0.0) += 1.0;
                }
            }
        }

        // Normalize frequencies
        let max_freq = freq.values().cloned().fold(0.0, f32::max);
        if max_freq > 0.0 {
            for value in freq.values_mut() {
                *value /= max_freq;
            }
        }

        freq
    }

    fn score_sentences(
        &self,
        sentences: &[&str],
        word_freq: &std::collections::HashMap<String, f32>,
    ) -> Vec<f32> {
        sentences
            .iter()
            .map(|sentence| {
                let words: Vec<String> = sentence
                    .split_whitespace()
                    .map(|w| {
                        w.to_lowercase()
                            .trim_matches(|c: char| !c.is_alphanumeric())
                            .to_string()
                    })
                    .filter(|w| !w.is_empty())
                    .collect();

                if words.is_empty() {
                    return 0.0;
                }

                let score: f32 = words.iter().map(|w| word_freq.get(w).copied().unwrap_or(0.0)).sum();
                score / words.len() as f32
            })
            .collect()
    }

    fn extract_action_items(&self, text: &str) -> Vec<String> {
        let action_keywords = ["need to", "should", "must", "will", "action item", "todo", "task"];
        let sentences = self.split_sentences(text);

        sentences
            .iter()
            .filter(|s| {
                let lower = s.to_lowercase();
                action_keywords.iter().any(|kw| lower.contains(kw))
            })
            .map(|s| s.trim().to_string())
            .take(5)
            .collect()
    }

    fn extract_decisions(&self, text: &str) -> Vec<String> {
        let decision_keywords = ["decided", "decision", "agreed", "conclusion", "resolved"];
        let sentences = self.split_sentences(text);

        sentences
            .iter()
            .filter(|s| {
                let lower = s.to_lowercase();
                decision_keywords.iter().any(|kw| lower.contains(kw))
            })
            .map(|s| s.trim().to_string())
            .take(5)
            .collect()
    }

    fn extract_key_points(&self, sentences: &[&str], selected_indices: &[usize]) -> Vec<String> {
        selected_indices
            .iter()
            .take(3)
            .map(|&i| sentences[i].trim().to_string())
            .collect()
    }
}

impl Default for RuleBasedSummarizer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rule_based_summarizer_extracts_structure() {
        let text = "The team decided to launch in March. We need to finish the billing \
                    integration before that. The launch date was agreed by everyone. \
                    Budget planning will continue next quarter.";
        let summarizer = RuleBasedSummarizer::new();
        let result = summarizer
            .summarize(text, SummarizationConfig::default())
            .unwrap();

        assert!(!result.summary.is_empty());
        assert!(result.decisions.iter().any(|d| d.contains("decided")));
        assert!(result.action_items.iter().any(|a| a.contains("need to")));
        assert_eq!(result.provider, "rule_based");
    }

    #[test]
    fn test_rule_based_handles_empty_input() {
        let summarizer = RuleBasedSummarizer::new();
        let result = summarizer
            .summarize("", SummarizationConfig::default())
            .unwrap();
        assert_eq!(result.summary, "");
        assert!(result.action_items.is_empty());
    }
}
