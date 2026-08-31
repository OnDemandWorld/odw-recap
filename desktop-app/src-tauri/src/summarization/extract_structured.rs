//! Structured extraction of action items and decisions from meeting transcripts.
//!
//! This module uses LLM providers to extract structured data from transcripts
//! in JSON format, which is then parsed into ActionItem and Decision records.

use crate::error::{RecapError, Result};
use crate::storage::types::{ActionItem, Decision};
use crate::summarization::LLMRouter;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Structured output from the LLM extraction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructuredExtraction {
    pub action_items: Vec<ExtractedActionItem>,
    pub decisions: Vec<ExtractedDecision>,
}

/// An action item extracted by the LLM
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedActionItem {
    pub description: String,
    #[serde(default)]
    pub assignee: Option<String>,
    #[serde(default)]
    pub deadline: Option<String>,
}

/// A decision extracted by the LLM
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedDecision {
    pub description: String,
    #[serde(default)]
    pub context: Option<String>,
    #[serde(default)]
    pub participants: Vec<String>,
}

/// Extract structured action items and decisions from a transcript using an LLM.
///
/// This function:
/// 1. Formats a prompt requesting JSON output
/// 2. Calls the LLM provider
/// 3. Parses the JSON response
/// 4. Converts to ActionItem/Decision records
pub async fn extract_structured_output(
    router: &LLMRouter,
    transcript: &str,
    meeting_id: Uuid,
    provider: Option<String>,
) -> Result<(Vec<ActionItem>, Vec<Decision>)> {
    // Build the extraction prompt
    let prompt = build_extraction_prompt(transcript);

    // Configure the LLM call
    let config = crate::summarization::SummarizationConfig {
        max_length: 4096, // Generous limit for JSON output
        include_action_items: true,
        include_decisions: true,
        include_key_points: false,
        prompt_template: None,
        model: None,
        api_key: None,
        use_rule_based: false,
        provider,
    };

    // Call the LLM
    let result = router.summarize(&prompt, config).await?;

    // Parse the JSON response
    let extraction = parse_extraction_response(&result.summary)?;

    // Convert to storage types
    let now = Utc::now().timestamp_millis();
    let action_items: Vec<ActionItem> = extraction
        .action_items
        .into_iter()
        .map(|item| ActionItem {
            id: Uuid::new_v4(),
            meeting_id,
            description: item.description,
            assignee: item.assignee,
            deadline: item.deadline,
            source_segment_id: None,
            status: "pending".to_string(),
            loop_task_id: None,
            sync_status: crate::storage::types::SyncStatus::NotSynced,
            created_at: now,
            updated_at: now,
        })
        .collect();

    let decisions: Vec<Decision> = extraction
        .decisions
        .into_iter()
        .map(|dec| Decision {
            id: Uuid::new_v4(),
            meeting_id,
            description: dec.description,
            context: dec.context,
            participants: dec.participants,
            source_segment_ids: vec![],
            vault_entry_id: None,
            sync_status: crate::storage::types::SyncStatus::NotSynced,
            created_at: now,
        })
        .collect();

    Ok((action_items, decisions))
}

/// Build a prompt that requests structured JSON output
fn build_extraction_prompt(transcript: &str) -> String {
    format!(
        r#"Analyze the following meeting transcript and extract all action items and decisions.

Return your response as a JSON object with the following structure:
{{
  "action_items": [
    {{
      "description": "Clear description of the action item",
      "assignee": "Person responsible (if mentioned)",
      "deadline": "Deadline if mentioned (optional)"
    }}
  ],
  "decisions": [
    {{
      "description": "Clear description of the decision made",
      "context": "Brief context for why this decision was made (optional)",
      "participants": ["List of people involved in making this decision"]
    }}
  ]
}}

If no action items or decisions are found, return empty arrays.
Only return the JSON object, no additional text or explanation.

Transcript:
{}

Extracted JSON:"#,
        transcript
    )
}

/// Parse the LLM's JSON response into structured extraction
fn parse_extraction_response(response: &str) -> Result<StructuredExtraction> {
    // Try to extract JSON from the response (LLMs sometimes add extra text)
    let json_str = extract_json_from_response(response)?;

    // Parse the JSON
    let extraction: StructuredExtraction = serde_json::from_str(&json_str).map_err(|e| {
        RecapError::Summarization(format!("Failed to parse extraction response: {}", e))
    })?;

    Ok(extraction)
}

/// Extract JSON object from LLM response (handles cases where LLM adds extra text)
fn extract_json_from_response(response: &str) -> Result<String> {
    // Try to find JSON object boundaries
    let trimmed = response.trim();

    // If it starts with { and ends with }, assume it's pure JSON
    if trimmed.starts_with('{') && trimmed.ends_with('}') {
        return Ok(trimmed.to_string());
    }

    // Otherwise, try to find JSON object in the response
    let start = trimmed.find('{').ok_or_else(|| {
        RecapError::Summarization("No JSON object found in response".to_string())
    })?;

    let end = trimmed.rfind('}').ok_or_else(|| {
        RecapError::Summarization("No JSON object found in response".to_string())
    })?;

    if start >= end {
        return Err(RecapError::Summarization(
            "Invalid JSON structure in response".to_string(),
        ));
    }

    Ok(trimmed[start..=end].to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_json_from_response_pure_json() {
        let response = r#"{"action_items": [], "decisions": []}"#;
        let result = extract_json_from_response(response).unwrap();
        assert_eq!(result, response);
    }

    #[test]
    fn test_extract_json_from_response_with_extra_text() {
        let response = r#"Here is the extracted JSON:
{"action_items": [{"description": "Test"}], "decisions": []}
Hope this helps!"#;
        let result = extract_json_from_response(response).unwrap();
        assert_eq!(
            result,
            r#"{"action_items": [{"description": "Test"}], "decisions": []}"#
        );
    }

    #[test]
    fn test_extract_json_from_response_no_json() {
        let response = "This is just text with no JSON";
        let result = extract_json_from_response(response);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_extraction_response() {
        let response = r#"{
            "action_items": [
                {
                    "description": "Review the proposal",
                    "assignee": "Alice",
                    "deadline": "Next Friday"
                }
            ],
            "decisions": [
                {
                    "description": "Approved the Q3 budget",
                    "context": "After reviewing the financial projections",
                    "participants": ["Bob", "Charlie"]
                }
            ]
        }"#;

        let extraction = parse_extraction_response(response).unwrap();
        assert_eq!(extraction.action_items.len(), 1);
        assert_eq!(extraction.action_items[0].description, "Review the proposal");
        assert_eq!(extraction.action_items[0].assignee, Some("Alice".to_string()));
        assert_eq!(extraction.decisions.len(), 1);
        assert_eq!(extraction.decisions[0].description, "Approved the Q3 budget");
        assert_eq!(extraction.decisions[0].participants.len(), 2);
    }

    #[test]
    fn test_parse_extraction_response_empty() {
        let response = r#"{"action_items": [], "decisions": []}"#;
        let extraction = parse_extraction_response(response).unwrap();
        assert_eq!(extraction.action_items.len(), 0);
        assert_eq!(extraction.decisions.len(), 0);
    }
}
