use serde::{Deserialize, Serialize};

/// Prompt template definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptTemplate {
    pub id: i64,
    pub name: String,
    pub description: String,
    pub content: String,
    pub variables: Vec<PromptVariable>,
    pub created_at: i64,
    pub updated_at: i64,
}

/// Variable definition for a prompt template
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptVariable {
    pub name: String,
    pub description: String,
    pub default_value: Option<String>,
    pub required: bool,
}

impl PromptTemplate {
    /// Create a new prompt template
    pub fn new(name: String, description: String, content: String) -> Self {
        let variables = Self::extract_variables(&content);
        Self {
            id: 0,
            name,
            description,
            content,
            variables,
            created_at: chrono::Utc::now().timestamp_millis(),
            updated_at: chrono::Utc::now().timestamp_millis(),
        }
    }

    /// Extract variables from template content
    pub fn extract_variables(content: &str) -> Vec<PromptVariable> {
        let mut variables = Vec::new();
        let mut seen = std::collections::HashSet::new();

        // Find all {{variable_name}} patterns
        let mut chars = content.chars().peekable();
        let mut current_var = String::new();
        let mut in_variable = false;

        while let Some(c) = chars.next() {
            if c == '{' && chars.peek() == Some(&'{') {
                chars.next(); // consume second {
                in_variable = true;
                current_var.clear();
            } else if c == '}' && in_variable && chars.peek() == Some(&'}') {
                chars.next(); // consume second }
                in_variable = false;

                let var_name = current_var.trim().to_string();
                if !var_name.is_empty() && !seen.contains(&var_name) {
                    seen.insert(var_name.clone());
                    variables.push(PromptVariable {
                        name: var_name,
                        description: String::new(),
                        default_value: None,
                        required: true,
                    });
                }
            } else if in_variable {
                current_var.push(c);
            }
        }

        variables
    }

    /// Validate that all required variables are provided
    pub fn validate_variables(
        &self,
        provided: &std::collections::HashMap<String, String>,
    ) -> std::result::Result<(), Vec<String>> {
        let mut missing = Vec::new();

        for var in &self.variables {
            if var.required && !provided.contains_key(&var.name) {
                if let Some(default) = &var.default_value {
                    // Has default, not missing
                    continue;
                }
                missing.push(var.name.clone());
            }
        }

        if missing.is_empty() {
            Ok(())
        } else {
            Err(missing)
        }
    }
}
