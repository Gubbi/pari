//! [`Role`] entity — a named actor (human or agent) with a kebab-case ID.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::schema::{
    ids::RoleId,
    store::EntityStore,
    types::Extensions,
    validation::{is_kebab_case, validate_extensions, ValidationError},
};

#[derive(Serialize, Deserialize, JsonSchema, pari_macros::Tracked)]
#[schemars(deny_unknown_fields)]
pub struct Role {
    pub id: RoleId,
    pub name: String,
    pub purpose: String,
    pub traits: Option<Vec<String>>,
    #[serde(flatten)]
    pub extensions: Extensions,
}

pub fn validate(role: &Role, _ctx: &EntityStore) -> Vec<ValidationError> {
    let mut errors = Vec::new();

    if !is_kebab_case(&role.id) {
        errors.push(ValidationError {
            path: "id".to_string(),
            message: format!("id must be kebab-case, got '{}'", role.id),
        });
    }

    errors.extend(validate_extensions(&role.extensions, "extensions"));

    errors
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::{store::EntityStore, types::Extensions};

    fn ctx() -> EntityStore {
        EntityStore::new()
    }

    fn valid_role() -> Role {
        Role {
            id: "eng-lead".into(),
            name: "Engineering Lead".to_string(),
            purpose: "Drive technical direction".to_string(),
            traits: None,
            extensions: Extensions::default(),
        }
    }

    // --- 4.1: Role struct and structural validator tests ---

    #[test]
    fn valid_role_passes_validation() {
        let errors = validate(&valid_role(), &ctx());
        assert!(errors.is_empty());
    }

    #[test]
    fn role_camel_case_id_fails() {
        let role = Role {
            id: "EngineeringLead".into(),
            ..valid_role()
        };
        let errors = validate(&role, &ctx());
        assert!(!errors.is_empty());
        assert_eq!(errors[0].path, "id");
        assert!(errors[0].message.contains("EngineeringLead"));
    }

    #[test]
    fn role_id_with_underscore_fails() {
        let role = Role {
            id: "eng_lead".into(),
            ..valid_role()
        };
        let errors = validate(&role, &ctx());
        assert!(!errors.is_empty());
        assert_eq!(errors[0].path, "id");
    }

    #[test]
    fn role_id_starting_with_digit_fails() {
        let role = Role {
            id: "1eng-lead".into(),
            ..valid_role()
        };
        let errors = validate(&role, &ctx());
        assert!(!errors.is_empty());
    }

    #[test]
    fn role_id_empty_fails() {
        let role = Role {
            id: "".into(),
            ..valid_role()
        };
        let errors = validate(&role, &ctx());
        assert!(!errors.is_empty());
    }

    #[test]
    fn role_with_traits_is_valid() {
        let role = Role {
            traits: Some(vec!["approver".to_string()]),
            ..valid_role()
        };
        let errors = validate(&role, &ctx());
        assert!(errors.is_empty());
    }

    #[test]
    fn role_without_traits_is_valid() {
        let role = Role {
            traits: None,
            ..valid_role()
        };
        let errors = validate(&role, &ctx());
        assert!(errors.is_empty());
    }

    // --- 8.2: Role extensions validation tests ---

    #[test]
    fn role_x_prefixed_extension_passes() {
        let mut map = std::collections::HashMap::new();
        map.insert("x-team".to_string(), serde_json::json!("platform"));
        let role = Role {
            extensions: Extensions(map),
            ..valid_role()
        };
        let errors = validate(&role, &ctx());
        assert!(errors.is_empty());
    }

    #[test]
    fn role_non_x_extension_key_fails() {
        let mut map = std::collections::HashMap::new();
        map.insert("team".to_string(), serde_json::json!("platform"));
        let role = Role {
            extensions: Extensions(map),
            ..valid_role()
        };
        let errors = validate(&role, &ctx());
        assert!(!errors.is_empty());
        assert!(errors[0].path.contains("extensions"));
        assert!(errors[0].message.contains("x-"));
    }
}
