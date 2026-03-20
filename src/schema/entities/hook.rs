//! [`Hook`] entity — a reusable automation action invoked at lifecycle points.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::schema::{
    ids::HookId,
    store::EntityStore,
    types::Extensions,
    validation::{is_camel_case, validate_extensions, ValidationError},
};

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct HookInput {
    pub name: String,
    pub description: String,
    pub required: bool,
}

#[derive(Serialize, Deserialize, JsonSchema)]
#[schemars(deny_unknown_fields)]
pub struct Hook {
    pub id: HookId,
    pub name: String,
    pub description: String,
    #[schemars(length(min = 1))]
    pub instructions: Vec<String>,
    pub inputs: Option<Vec<HookInput>>,
    #[serde(flatten)]
    pub extensions: Extensions,
}

pub fn validate(hook: &Hook, _ctx: &EntityStore) -> Vec<ValidationError> {
    let mut errors = Vec::new();

    if !is_camel_case(&hook.id) {
        errors.push(ValidationError {
            path: "id".to_string(),
            message: format!("id must be CamelCase, got '{}'", hook.id),
        });
    }

    if hook.instructions.is_empty() {
        errors.push(ValidationError {
            path: "instructions".to_string(),
            message: "instructions must have at least one item".to_string(),
        });
    }

    errors.extend(validate_extensions(&hook.extensions, "extensions"));

    errors
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::types::Extensions;

    fn ctx() -> EntityStore {
        EntityStore::new()
    }

    fn valid_hook() -> Hook {
        Hook {
            id: "UpdateJiraStatus".into(),
            name: "Update Jira Status".to_string(),
            description: "Updates the Jira issue status".to_string(),
            instructions: vec!["Call the Jira API".to_string()],
            inputs: None,
            extensions: Extensions::default(),
        }
    }

    // --- 5.1: Hook struct and structural validator tests ---

    #[test]
    fn valid_hook_passes_validation() {
        let errors = validate(&valid_hook(), &ctx());
        assert!(errors.is_empty());
    }

    #[test]
    fn hook_with_inputs_is_valid() {
        let hook = Hook {
            inputs: Some(vec![HookInput {
                name: "status".to_string(),
                description: "The new status".to_string(),
                required: true,
            }]),
            ..valid_hook()
        };
        let errors = validate(&hook, &ctx());
        assert!(errors.is_empty());
    }

    #[test]
    fn hook_without_inputs_is_valid() {
        let errors = validate(&valid_hook(), &ctx());
        assert!(errors.is_empty());
    }

    #[test]
    fn hook_kebab_id_fails() {
        let hook = Hook {
            id: "update-jira".into(),
            ..valid_hook()
        };
        let errors = validate(&hook, &ctx());
        assert!(!errors.is_empty());
        assert_eq!(errors[0].path, "id");
    }

    #[test]
    fn hook_lowercase_id_fails() {
        let hook = Hook {
            id: "updateJira".into(),
            ..valid_hook()
        };
        let errors = validate(&hook, &ctx());
        assert!(!errors.is_empty());
        assert_eq!(errors[0].path, "id");
    }

    #[test]
    fn hook_empty_instructions_fails() {
        let hook = Hook {
            instructions: vec![],
            ..valid_hook()
        };
        let errors = validate(&hook, &ctx());
        assert!(!errors.is_empty());
        assert_eq!(errors[0].path, "instructions");
    }

    #[test]
    fn hook_missing_instructions_field_represented_as_empty_vec_fails() {
        let hook = Hook {
            instructions: vec![],
            ..valid_hook()
        };
        let errors = validate(&hook, &ctx());
        assert!(errors.iter().any(|e| e.path == "instructions"));
    }

    #[test]
    fn hook_input_shape_has_required_fields() {
        let input = HookInput {
            name: "status".to_string(),
            description: "The status value".to_string(),
            required: true,
        };
        assert_eq!(input.name, "status");
        assert!(input.required);
    }

    // --- 8.2: Hook extensions validation tests ---

    #[test]
    fn hook_x_prefixed_extension_passes() {
        let mut map = std::collections::HashMap::new();
        map.insert("x-owner".to_string(), serde_json::json!("platform"));
        let hook = Hook {
            extensions: Extensions(map),
            ..valid_hook()
        };
        let errors = validate(&hook, &ctx());
        assert!(errors.is_empty());
    }

    #[test]
    fn hook_non_x_extension_key_fails() {
        let mut map = std::collections::HashMap::new();
        map.insert("owner".to_string(), serde_json::json!("platform"));
        let hook = Hook {
            extensions: Extensions(map),
            ..valid_hook()
        };
        let errors = validate(&hook, &ctx());
        assert!(!errors.is_empty());
        assert!(errors[0].path.contains("extensions"));
        assert!(errors[0].message.contains("x-"));
    }
}
