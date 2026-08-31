use crate::error::Result;
use std::collections::HashMap;

/// Substitute variables in a template string
#[allow(dead_code)]
pub fn substitute_variables(template: &str, variables: &HashMap<String, String>) -> Result<String> {
    let mut result = String::new();
    let mut chars = template.chars().peekable();
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
            if let Some(value) = variables.get(&var_name) {
                result.push_str(value);
            } else {
                // Variable not provided, keep original placeholder
                result.push_str(&format!("{{{{{}}}}}", var_name));
            }
        } else if in_variable {
            current_var.push(c);
        } else {
            result.push(c);
        }
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_substitute_variables() {
        let template = "Hello {{name}}, you have {{count}} messages.";
        let mut variables = HashMap::new();
        variables.insert("name".to_string(), "Alice".to_string());
        variables.insert("count".to_string(), "5".to_string());

        let result = substitute_variables(template, &variables).unwrap();
        assert_eq!(result, "Hello Alice, you have 5 messages.");
    }

    #[test]
    fn test_missing_variable() {
        let template = "Hello {{name}}!";
        let variables = HashMap::new();

        let result = substitute_variables(template, &variables).unwrap();
        assert_eq!(result, "Hello {{name}}!");
    }

    #[test]
    fn test_multiple_occurrences() {
        let template = "{{word}} and {{word}} again";
        let mut variables = HashMap::new();
        variables.insert("word".to_string(), "test".to_string());

        let result = substitute_variables(template, &variables).unwrap();
        assert_eq!(result, "test and test again");
    }
}
