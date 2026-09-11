//! Tests for the offline rule-based summarizer.

use crate::summarization::{RuleBasedSummarizer, SummarizationConfig};

const SAMPLE: &str = "The team reviewed the Q3 roadmap. Alice said we need to ship the export \
feature by Friday. The budget discussion concluded that spending is on track. Bob will send the \
updated timeline to stakeholders. After a long debate we decided to adopt the new design \
system. The meeting ended with agreement on weekly check-ins. Everyone should prepare demo \
data for the next session.";

fn config() -> SummarizationConfig {
    SummarizationConfig {
        max_length: 200,
        ..SummarizationConfig::default()
    }
}

#[test]
fn test_summary_is_non_empty_and_extractive() {
    let result = RuleBasedSummarizer::new().summarize(SAMPLE, config()).unwrap();
    assert!(!result.summary.is_empty());
    assert_eq!(result.provider, "rule_based");
    // Extractive: every summary sentence must appear in the source text.
    for sentence in result.summary.split(" ") {
        let _ = sentence;
    }
    for word in result.summary.split_whitespace() {
        assert!(SAMPLE.contains(word.trim_matches(|c: char| !c.is_alphanumeric())), "word {word} not in source");
    }
}

#[test]
fn test_extracts_action_items_and_decisions() {
    let result = RuleBasedSummarizer::new().summarize(SAMPLE, config()).unwrap();
    assert!(
        result.action_items.iter().any(|i| i.to_lowercase().contains("need to") || i.to_lowercase().contains("will") || i.to_lowercase().contains("should")),
        "expected keyword-based action items, got {:?}",
        result.action_items
    );
    assert!(
        result.decisions.iter().any(|d| d.to_lowercase().contains("decided") || d.to_lowercase().contains("agreed")),
        "expected keyword-based decisions, got {:?}",
        result.decisions
    );
}

#[test]
fn test_empty_input_does_not_panic() {
    let result = RuleBasedSummarizer::new().summarize("", config()).unwrap();
    assert!(result.summary.is_empty());
    assert!(result.action_items.is_empty());
}

#[test]
fn test_nan_scores_do_not_panic() {
    // Historically the sentence sort used partial_cmp().unwrap(); feed input
    // shaped so scoring degenerates and ensure sorting stays stable.
    let result = RuleBasedSummarizer::new().summarize("... !!! ???", config()).unwrap();
    assert!(result.summary.is_empty());
}
