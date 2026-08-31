pub mod template;
pub mod variable_substitution;

use crate::error::Result;
use crate::storage::StorageManager;
use std::sync::Arc;
use std::sync::Mutex;

pub use template::{PromptTemplate, PromptVariable};
pub use variable_substitution::substitute_variables;

/// Prompt manager for handling prompt templates
pub struct PromptManager {
    storage: Arc<Mutex<StorageManager>>,
}

impl PromptManager {
    pub fn new(storage: Arc<Mutex<StorageManager>>) -> Self {
        Self { storage }
    }

    /// Get a prompt template by name
    pub fn get_template(&self, name: &str) -> Result<Option<PromptTemplate>> {
        let storage = self.storage.lock().map_err(|_| {
            crate::error::RecapError::Storage("Failed to lock storage".to_string())
        })?;
        let result = storage.get_prompt_template(name)?;
        Ok(result.map(|(content, _meeting_type, _is_custom)| {
            let variables = PromptTemplate::extract_variables(&content);
            PromptTemplate {
                id: 0, // ID not stored in current schema
                name: name.to_string(),
                description: String::new(),
                content,
                variables,
                created_at: chrono::Utc::now().timestamp_millis(),
                updated_at: chrono::Utc::now().timestamp_millis(),
            }
        }))
    }

    /// Save a prompt template
    pub fn save_template(&self, template: &PromptTemplate) -> Result<()> {
        let storage = self.storage.lock().map_err(|_| {
            crate::error::RecapError::Storage("Failed to lock storage".to_string())
        })?;
        storage.save_prompt_template(&template.name, &template.content, None, true)
    }

    /// List all prompt templates
    pub fn list_templates(&self) -> Result<Vec<PromptTemplate>> {
        let storage = self.storage.lock().map_err(|_| {
            crate::error::RecapError::Storage("Failed to lock storage".to_string())
        })?;
        let rows = storage.list_prompt_templates()?;
        Ok(rows
            .into_iter()
            .map(|(name, content, _meeting_type, _is_custom)| {
                let variables = PromptTemplate::extract_variables(&content);
                PromptTemplate {
                    id: 0, // autoincrement id not surfaced through StorageManager yet
                    name,
                    description: String::new(),
                    content,
                    variables,
                    created_at: chrono::Utc::now().timestamp_millis(),
                    updated_at: chrono::Utc::now().timestamp_millis(),
                }
            })
            .collect())
    }

    /// Render a prompt template with variable substitution
    #[allow(dead_code)]
    pub fn render_template(
        &self,
        template_name: &str,
        variables: std::collections::HashMap<String, String>,
    ) -> Result<String> {
        let template = self
            .get_template(template_name)?
            .ok_or_else(|| crate::error::RecapError::Config("Template not found".to_string()))?;

        substitute_variables(&template.content, &variables)
    }

    /// Create default prompt templates
    pub fn initialize_default_templates(&self) -> Result<()> {
        let default_templates = vec![
            PromptTemplate {
                id: 1,
                name: "Meeting Summary".to_string(),
                description: "Generate a concise summary of the meeting".to_string(),
                content: "Summarize the following meeting transcript in {{max_length}} words or less. Focus on key points, decisions made, and action items.\n\nTranscript:\n{{transcript}}".to_string(),
                variables: vec![
                    PromptVariable {
                        name: "max_length".to_string(),
                        description: "Maximum summary length in words".to_string(),
                        default_value: Some("500".to_string()),
                        required: true,
                    },
                    PromptVariable {
                        name: "transcript".to_string(),
                        description: "The meeting transcript text".to_string(),
                        default_value: None,
                        required: true,
                    },
                ],
                created_at: chrono::Utc::now().timestamp_millis(),
                updated_at: chrono::Utc::now().timestamp_millis(),
            },
            PromptTemplate {
                id: 2,
                name: "Action Items Extraction".to_string(),
                description: "Extract action items from meeting transcript".to_string(),
                content: "Extract all action items from the following meeting transcript. List them as bullet points with assignee (if mentioned) and deadline (if mentioned).\n\nTranscript:\n{{transcript}}".to_string(),
                variables: vec![PromptVariable {
                    name: "transcript".to_string(),
                    description: "The meeting transcript text".to_string(),
                    default_value: None,
                    required: true,
                }],
                created_at: chrono::Utc::now().timestamp_millis(),
                updated_at: chrono::Utc::now().timestamp_millis(),
            },
            PromptTemplate {
                id: 3,
                name: "Key Decisions".to_string(),
                description: "Extract key decisions from meeting transcript".to_string(),
                content: "Identify and list all key decisions made during the following meeting. For each decision, include the context and rationale if available.\n\nTranscript:\n{{transcript}}".to_string(),
                variables: vec![PromptVariable {
                    name: "transcript".to_string(),
                    description: "The meeting transcript text".to_string(),
                    default_value: None,
                    required: true,
                }],
                created_at: chrono::Utc::now().timestamp_millis(),
                updated_at: chrono::Utc::now().timestamp_millis(),
            },
        ];

        for template in default_templates {
            // Only seed templates that don't exist yet so user edits to the
            // defaults survive restarts.
            if self.get_template(&template.name)?.is_none() {
                self.save_template(&template)?;
            }
        }

        Ok(())
    }
}
