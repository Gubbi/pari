use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::schema::context::RepoContext;
use crate::schema::validation::{is_kebab_case, ValidationError};

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct Role {
    #[schemars(regex(pattern = r"^[a-z][a-z0-9-]*$"))]
    pub id: String,
    pub name: String,
    pub purpose: String,
    pub traits: Option<Vec<String>>,
}

pub fn validate(role: &Role, _ctx: &RepoContext) -> Vec<ValidationError> {
    let mut errors = Vec::new();

    if !is_kebab_case(&role.id) {
        errors.push(ValidationError {
            path: "id".to_string(),
            message: format!("id must be kebab-case, got '{}'", role.id),
        });
    }

    errors
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx() -> RepoContext {
        RepoContext::new()
    }

    fn valid_role() -> Role {
        Role {
            id: "eng-lead".to_string(),
            name: "Engineering Lead".to_string(),
            purpose: "Drive technical direction".to_string(),
            traits: None,
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
            id: "EngineeringLead".to_string(),
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
            id: "eng_lead".to_string(),
            ..valid_role()
        };
        let errors = validate(&role, &ctx());
        assert!(!errors.is_empty());
        assert_eq!(errors[0].path, "id");
    }

    #[test]
    fn role_id_starting_with_digit_fails() {
        let role = Role {
            id: "1eng-lead".to_string(),
            ..valid_role()
        };
        let errors = validate(&role, &ctx());
        assert!(!errors.is_empty());
    }

    #[test]
    fn role_id_empty_fails() {
        let role = Role {
            id: "".to_string(),
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

}
