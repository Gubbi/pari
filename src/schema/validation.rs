//! Cross-entity validation helpers and [`ValidationError`].
//!
//! Each entity module calls into these shared validators for RACI, hooks, and extensions.

use std::collections::HashMap;

use crate::schema::{
    store::EntityStore,
    types::{Extensions, HookInvocation, HooksMap, Raci},
};

// --- ValidationError ---

#[derive(Debug, thiserror::Error)]
#[error("{message} at {path}")]
pub struct ValidationError {
    pub path: String,
    pub message: String,
}

// --- Id format helpers ---

/// Returns true if `s` matches `^[a-z][a-z0-9-]*$`.
pub fn is_kebab_case(s: &str) -> bool {
    let Some(first) = s.chars().next() else {
        return false;
    };
    if !first.is_ascii_lowercase() {
        return false;
    }
    s.chars()
        .skip(1)
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

/// Returns true if `s` matches `^[A-Z][A-Za-z0-9]*$`.
pub fn is_camel_case(s: &str) -> bool {
    let Some(first) = s.chars().next() else {
        return false;
    };
    if !first.is_ascii_uppercase() {
        return false;
    }
    s.chars().skip(1).all(|c| c.is_ascii_alphanumeric())
}

/// Returns true if `s` matches `@[a-z0-9._-]+`.
pub fn is_valid_handle(s: &str) -> bool {
    let Some(rest) = s.strip_prefix('@') else {
        return false;
    };
    if rest.is_empty() {
        return false;
    }
    rest.chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '.' || c == '_' || c == '-')
}

// --- Shared cross-entity validators ---

/// Validates that all keys in `extensions` match `^x-`.
/// Called during Phase 1 (structural) validation for every entity type.
pub fn validate_extensions(extensions: &Extensions, path: &str) -> Vec<ValidationError> {
    extensions
        .0
        .keys()
        .filter(|k| !k.starts_with("x-"))
        .map(|k| ValidationError {
            path: format!("{path}.{k}"),
            message: format!("extension key '{k}' must be prefixed with 'x-'"),
        })
        .collect()
}

/// Validates that all `role_ids` in a RACI block exist in the entity store.
pub fn validate_raci(raci: &Raci, path: &str, ctx: &EntityStore) -> Vec<ValidationError> {
    let mut errors = Vec::new();

    if !ctx.has_role(&raci.responsible) {
        errors.push(ValidationError {
            path: format!("{path}.responsible"),
            message: format!("unknown role '{}'", raci.responsible),
        });
    }
    if !ctx.has_role(&raci.accountable) {
        errors.push(ValidationError {
            path: format!("{path}.accountable"),
            message: format!("unknown role '{}'", raci.accountable),
        });
    }
    for (i, role_id) in raci.consulted.iter().enumerate() {
        if !ctx.has_role(role_id) {
            errors.push(ValidationError {
                path: format!("{path}.consulted[{i}]"),
                message: format!("unknown role '{role_id}'"),
            });
        }
    }
    for (i, role_id) in raci.informed.iter().enumerate() {
        if !ctx.has_role(role_id) {
            errors.push(ValidationError {
                path: format!("{path}.informed[{i}]"),
                message: format!("unknown role '{role_id}'"),
            });
        }
    }

    errors
}

/// Validates referential integrity and input correctness for all hook invocations in a `HooksMap`.
pub fn validate_hooks_map(hooks: &HooksMap, path: &str, ctx: &EntityStore) -> Vec<ValidationError> {
    let mut errors = Vec::new();

    for (lifecycle_point, invocation_value) in hooks {
        let invocations = invocation_value.invocations();
        for (i, inv) in invocations.iter().enumerate() {
            let inv_path = if invocations.len() == 1 {
                format!("{path}.{lifecycle_point}")
            } else {
                format!("{path}.{lifecycle_point}[{i}]")
            };

            let hook_id = inv.hook_id();

            // Referential integrity
            if !ctx.has_hook(hook_id) {
                errors.push(ValidationError {
                    path: inv_path.clone(),
                    message: format!("unknown hook '{hook_id}'"),
                });
            }

            // Input validation (only for Object invocations with `with`)
            if let HookInvocation::Object { hook, with } = inv {
                if let Some(hook_entity) = ctx.get_hook(hook) {
                    if let Some(inputs) = &hook_entity.inputs {
                        errors.extend(validate_hook_invocation_inputs(with, inputs, &inv_path));
                    }
                }
            }
        }
    }

    errors
}

#[allow(clippy::ref_option)]
fn validate_hook_invocation_inputs(
    with: &Option<HashMap<String, String>>,
    inputs: &[crate::schema::entities::hook::HookInput],
    path: &str,
) -> Vec<ValidationError> {
    let mut errors = Vec::new();
    let empty = HashMap::new();
    let provided = with.as_ref().unwrap_or(&empty);

    // Check required inputs are present
    for input in inputs {
        if input.required && !provided.contains_key(&input.name) {
            errors.push(ValidationError {
                path: format!("{path}.with"),
                message: format!("missing required input '{}'", input.name),
            });
        }
    }

    // Check no unknown keys
    let declared_names: std::collections::HashSet<&str> =
        inputs.iter().map(|i| i.name.as_str()).collect();
    for key in provided.keys() {
        if !declared_names.contains(key.as_str()) {
            errors.push(ValidationError {
                path: format!("{path}.with.{key}"),
                message: format!("unknown input key '{key}'"),
            });
        }
    }

    errors
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;
    use crate::schema::{
        entities::{
            hook::{Hook, HookInput},
            role::Role,
        },
        store::EntityStore,
        types::{Extensions, HookInvocationValue, Raci},
    };

    // --- validate_extensions tests ---

    #[test]
    fn extensions_all_x_prefix_passes() {
        let mut map = HashMap::new();
        map.insert("x-team".to_string(), serde_json::json!("platform"));
        map.insert("x-sla".to_string(), serde_json::json!("24h"));
        let ext = Extensions(map);
        let errors = validate_extensions(&ext, "extensions");
        assert!(errors.is_empty());
    }

    #[test]
    fn extensions_empty_passes() {
        let ext = Extensions(HashMap::new());
        let errors = validate_extensions(&ext, "extensions");
        assert!(errors.is_empty());
    }

    #[test]
    fn extensions_non_prefixed_key_fails() {
        let mut map = HashMap::new();
        map.insert("team".to_string(), serde_json::json!("platform"));
        let ext = Extensions(map);
        let errors = validate_extensions(&ext, "extensions");
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].path, "extensions.team");
        assert!(errors[0].message.contains("x-"));
    }

    #[test]
    fn extensions_mixed_keys_only_invalid_reported() {
        let mut map = HashMap::new();
        map.insert("x-good".to_string(), serde_json::json!(true));
        map.insert("bad".to_string(), serde_json::json!(true));
        let ext = Extensions(map);
        let errors = validate_extensions(&ext, "extensions");
        assert_eq!(errors.len(), 1);
        assert!(errors[0].path.contains("bad"));
    }

    fn ctx_with_roles(role_ids: &[&str]) -> EntityStore {
        let mut ctx = EntityStore::new();
        for id in role_ids {
            ctx.roles.insert(
                id.to_string(),
                Role {
                    id: (*id).into(),
                    name: id.to_string(),
                    purpose: "test".to_string(),
                    traits: None,
                    extensions: Extensions::default(),
                },
            );
        }
        ctx
    }

    fn ctx_with_hooks(hook_ids: &[&str]) -> EntityStore {
        let mut ctx = EntityStore::new();
        for id in hook_ids {
            ctx.hooks.insert(
                id.to_string(),
                Hook {
                    id: (*id).into(),
                    name: id.to_string(),
                    description: "test".to_string(),
                    instructions: vec!["do it".to_string()],
                    inputs: None,
                    extensions: Extensions::default(),
                },
            );
        }
        ctx
    }

    fn ctx_with_hook_def(hook_id: &str, inputs: Vec<(&str, bool)>) -> EntityStore {
        let mut ctx = EntityStore::new();
        ctx.hooks.insert(
            hook_id.to_string(),
            Hook {
                id: hook_id.into(),
                name: hook_id.to_string(),
                description: "test".to_string(),
                instructions: vec!["do it".to_string()],
                inputs: Some(
                    inputs
                        .into_iter()
                        .map(|(name, required)| HookInput {
                            name: name.to_string(),
                            description: "desc".to_string(),
                            required,
                        })
                        .collect(),
                ),
                extensions: Extensions::default(),
            },
        );
        ctx
    }

    // --- 5.1: ValidationError Display and std::error::Error tests ---

    #[test]
    fn validation_error_display_format() {
        let err = ValidationError {
            path: "state_map.Active".to_string(),
            message: "unknown state".to_string(),
        };
        assert_eq!(format!("{}", err), "unknown state at state_map.Active");
    }

    #[test]
    fn validation_error_implements_std_error() {
        let err = ValidationError {
            path: "id".to_string(),
            message: "id must be CamelCase".to_string(),
        };
        let _: &dyn std::error::Error = &err;
    }

    // --- 3.1: is_kebab_case tests ---

    #[test]
    fn kebab_valid_simple() {
        assert!(is_kebab_case("foo"));
    }

    #[test]
    fn kebab_valid_with_numbers() {
        assert!(is_kebab_case("foo-123"));
    }

    #[test]
    fn kebab_valid_multi_segment() {
        assert!(is_kebab_case("platform-team"));
    }

    #[test]
    fn kebab_valid_single_char() {
        assert!(is_kebab_case("a"));
    }

    #[test]
    fn kebab_invalid_uppercase_start() {
        assert!(!is_kebab_case("Foo"));
    }

    #[test]
    fn kebab_invalid_camel_case() {
        assert!(!is_kebab_case("FooBar"));
    }

    #[test]
    fn kebab_invalid_underscore() {
        assert!(!is_kebab_case("foo_bar"));
    }

    #[test]
    fn kebab_invalid_starts_with_digit() {
        assert!(!is_kebab_case("1foo"));
    }

    #[test]
    fn kebab_invalid_empty() {
        assert!(!is_kebab_case(""));
    }

    #[test]
    fn kebab_invalid_leading_dash() {
        assert!(!is_kebab_case("-foo"));
    }

    // --- 3.1: is_camel_case tests ---

    #[test]
    fn camel_valid_simple() {
        assert!(is_camel_case("Foo"));
    }

    #[test]
    fn camel_valid_compound() {
        assert!(is_camel_case("FooBar"));
    }

    #[test]
    fn camel_valid_with_numbers() {
        assert!(is_camel_case("Foo123"));
    }

    #[test]
    fn camel_valid_single_char() {
        assert!(is_camel_case("A"));
    }

    #[test]
    fn camel_invalid_lowercase_start() {
        assert!(!is_camel_case("foo"));
    }

    #[test]
    fn camel_invalid_kebab() {
        assert!(!is_camel_case("foo-bar"));
    }

    #[test]
    fn camel_invalid_underscore() {
        assert!(!is_camel_case("Foo_Bar"));
    }

    #[test]
    fn camel_invalid_empty() {
        assert!(!is_camel_case(""));
    }

    // --- 14.1: Hook invocation input validator tests ---

    #[test]
    fn hook_invocation_all_required_inputs_provided() {
        let ctx = ctx_with_hook_def("UpdateJira", vec![("status", true), ("comment", false)]);
        let mut with = HashMap::new();
        with.insert("status".to_string(), "Done".to_string());
        let mut hooks: HooksMap = HashMap::new();
        hooks.insert(
            "after".to_string(),
            HookInvocationValue::Single(HookInvocation::Object {
                hook: "UpdateJira".to_string(),
                with: Some(with),
            }),
        );
        let errors = validate_hooks_map(&hooks, "hooks", &ctx);
        assert!(
            errors.is_empty(),
            "Expected no errors, got: {:?}",
            errors.iter().map(|e| &e.message).collect::<Vec<_>>()
        );
    }

    #[test]
    fn hook_invocation_missing_required_input() {
        let ctx = ctx_with_hook_def("UpdateJira", vec![("status", true)]);
        let mut hooks: HooksMap = HashMap::new();
        hooks.insert(
            "after".to_string(),
            HookInvocationValue::Single(HookInvocation::Object {
                hook: "UpdateJira".to_string(),
                with: None,
            }),
        );
        let errors = validate_hooks_map(&hooks, "hooks", &ctx);
        assert!(!errors.is_empty());
        assert!(errors[0]
            .message
            .contains("missing required input 'status'"));
    }

    #[test]
    fn hook_invocation_unknown_key_in_with() {
        let ctx = ctx_with_hook_def("UpdateJira", vec![("status", true)]);
        let mut with = HashMap::new();
        with.insert("status".to_string(), "Done".to_string());
        with.insert("unknown_key".to_string(), "value".to_string());
        let mut hooks: HooksMap = HashMap::new();
        hooks.insert(
            "after".to_string(),
            HookInvocationValue::Single(HookInvocation::Object {
                hook: "UpdateJira".to_string(),
                with: Some(with),
            }),
        );
        let errors = validate_hooks_map(&hooks, "hooks", &ctx);
        assert!(!errors.is_empty());
        assert!(errors
            .iter()
            .any(|e| e.message.contains("unknown input key")));
    }

    #[test]
    fn hooks_map_unknown_hook_id_fails_referential_integrity() {
        let ctx = EntityStore::new();
        let mut hooks: HooksMap = HashMap::new();
        hooks.insert(
            "after".to_string(),
            HookInvocationValue::Single(HookInvocation::Bare("UnknownHook".to_string())),
        );
        let errors = validate_hooks_map(&hooks, "hooks", &ctx);
        assert!(!errors.is_empty());
        assert!(errors[0].message.contains("unknown hook 'UnknownHook'"));
    }

    #[test]
    fn hooks_map_known_hook_id_passes_referential_integrity() {
        let ctx = ctx_with_hooks(&["NotifySlack"]);
        let mut hooks: HooksMap = HashMap::new();
        hooks.insert(
            "after".to_string(),
            HookInvocationValue::Single(HookInvocation::Bare("NotifySlack".to_string())),
        );
        let errors = validate_hooks_map(&hooks, "hooks", &ctx);
        assert!(errors.is_empty());
    }

    // validate_raci integration tests
    #[test]
    fn raci_valid_all_roles_exist() {
        let ctx = ctx_with_roles(&["eng-lead", "pm", "designer"]);
        let raci = Raci {
            responsible: "eng-lead".to_string(),
            accountable: "pm".to_string(),
            consulted: vec!["designer".to_string()],
            informed: vec![],
        };
        let errors = validate_raci(&raci, "accountability", &ctx);
        assert!(errors.is_empty());
    }

    #[test]
    fn raci_unknown_responsible_fails() {
        let ctx = ctx_with_roles(&["pm"]);
        let raci = Raci {
            responsible: "unknown-role".to_string(),
            accountable: "pm".to_string(),
            consulted: vec![],
            informed: vec![],
        };
        let errors = validate_raci(&raci, "accountability", &ctx);
        assert!(!errors.is_empty());
        assert!(errors[0].path.contains("responsible"));
    }

    #[test]
    fn raci_unknown_role_in_consulted_fails() {
        let ctx = ctx_with_roles(&["eng-lead", "pm"]);
        let raci = Raci {
            responsible: "eng-lead".to_string(),
            accountable: "pm".to_string(),
            consulted: vec!["unknown-role".to_string()],
            informed: vec![],
        };
        let errors = validate_raci(&raci, "accountability", &ctx);
        assert!(!errors.is_empty());
        assert!(errors[0].path.contains("consulted[0]"));
    }
}
