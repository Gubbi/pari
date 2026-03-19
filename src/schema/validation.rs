use std::collections::HashMap;

use crate::schema::context::RepoContext;
use crate::schema::types::{HookInvocation, HooksMap, Raci};

// --- ValidationError ---

pub struct ValidationError {
    pub path: String,
    pub message: String,
}

// --- Id format helpers ---

/// Returns true if `s` matches `^[a-z][a-z0-9-]*$`.
pub fn is_kebab_case(s: &str) -> bool {
    if s.is_empty() {
        return false;
    }
    let mut chars = s.chars();
    let first = chars.next().unwrap();
    if !first.is_ascii_lowercase() {
        return false;
    }
    chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

/// Returns true if `s` matches `^[A-Z][A-Za-z0-9]*$`.
pub fn is_camel_case(s: &str) -> bool {
    if s.is_empty() {
        return false;
    }
    let mut chars = s.chars();
    let first = chars.next().unwrap();
    if !first.is_ascii_uppercase() {
        return false;
    }
    chars.all(|c| c.is_ascii_alphanumeric())
}

/// Returns true if `s` matches `@[a-z0-9._-]+`.
pub fn is_valid_handle(s: &str) -> bool {
    let rest = match s.strip_prefix('@') {
        Some(r) => r,
        None => return false,
    };
    if rest.is_empty() {
        return false;
    }
    rest.chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '.' || c == '_' || c == '-')
}

// --- Shared cross-entity validators ---

/// Validates that all role_ids in a RACI block exist in the repository context.
pub fn validate_raci(raci: &Raci, path: &str, ctx: &RepoContext) -> Vec<ValidationError> {
    let mut errors = Vec::new();

    if !ctx.has_role(&raci.responsible) {
        errors.push(ValidationError {
            path: format!("{}.responsible", path),
            message: format!("unknown role '{}'", raci.responsible),
        });
    }
    if !ctx.has_role(&raci.accountable) {
        errors.push(ValidationError {
            path: format!("{}.accountable", path),
            message: format!("unknown role '{}'", raci.accountable),
        });
    }
    for (i, role_id) in raci.consulted.iter().enumerate() {
        if !ctx.has_role(role_id) {
            errors.push(ValidationError {
                path: format!("{}.consulted[{}]", path, i),
                message: format!("unknown role '{}'", role_id),
            });
        }
    }
    for (i, role_id) in raci.informed.iter().enumerate() {
        if !ctx.has_role(role_id) {
            errors.push(ValidationError {
                path: format!("{}.informed[{}]", path, i),
                message: format!("unknown role '{}'", role_id),
            });
        }
    }

    errors
}

/// Validates referential integrity and input correctness for all hook invocations in a HooksMap.
pub fn validate_hooks_map(
    hooks: &HooksMap,
    path: &str,
    ctx: &RepoContext,
) -> Vec<ValidationError> {
    let mut errors = Vec::new();

    for (lifecycle_point, invocation_value) in hooks {
        let invocations = invocation_value.invocations();
        for (i, inv) in invocations.iter().enumerate() {
            let inv_path = if invocations.len() == 1 {
                format!("{}.{}", path, lifecycle_point)
            } else {
                format!("{}.{}[{}]", path, lifecycle_point, i)
            };

            let hook_id = inv.hook_id();

            // Referential integrity
            if !ctx.has_hook(hook_id) {
                errors.push(ValidationError {
                    path: inv_path.clone(),
                    message: format!("unknown hook '{}'", hook_id),
                });
            }

            // Input validation (only for Object invocations with `with`)
            if let HookInvocation::Object { hook, with } = inv {
                if let Some(def) = ctx.get_hook_definition(hook) {
                    errors.extend(validate_hook_invocation_inputs(with, def, &inv_path, ctx));
                }
            }
        }
    }

    errors
}

fn validate_hook_invocation_inputs(
    with: &Option<HashMap<String, String>>,
    def: &crate::schema::context::HookDefinition,
    path: &str,
    _ctx: &RepoContext,
) -> Vec<ValidationError> {
    let mut errors = Vec::new();
    let empty = HashMap::new();
    let provided = with.as_ref().unwrap_or(&empty);

    // Check required inputs are present
    for input in &def.inputs {
        if input.required && !provided.contains_key(&input.name) {
            errors.push(ValidationError {
                path: format!("{}.with", path),
                message: format!("missing required input '{}'", input.name),
            });
        }
    }

    // Check no unknown keys
    let declared_names: std::collections::HashSet<&str> =
        def.inputs.iter().map(|i| i.name.as_str()).collect();
    for key in provided.keys() {
        if !declared_names.contains(key.as_str()) {
            errors.push(ValidationError {
                path: format!("{}.with.{}", path, key),
                message: format!("unknown input key '{}'", key),
            });
        }
    }

    errors
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::context::{HookDefinition, HookInputInfo, RepoContext};
    use crate::schema::types::{HookInvocationValue, Raci};
    use std::collections::HashMap;

    fn ctx_with_roles(roles: &[&str]) -> RepoContext {
        let mut ctx = RepoContext::new();
        for r in roles {
            ctx.role_ids.insert(r.to_string());
        }
        ctx
    }

    fn ctx_with_hooks(hooks: &[&str]) -> RepoContext {
        let mut ctx = RepoContext::new();
        for h in hooks {
            ctx.hook_ids.insert(h.to_string());
        }
        ctx
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

    fn ctx_with_hook_def(
        hook_id: &str,
        inputs: Vec<(&str, bool)>,
    ) -> RepoContext {
        let mut ctx = RepoContext::new();
        ctx.hook_ids.insert(hook_id.to_string());
        ctx.hook_definitions.insert(
            hook_id.to_string(),
            HookDefinition {
                inputs: inputs
                    .into_iter()
                    .map(|(name, required)| HookInputInfo {
                        name: name.to_string(),
                        required,
                    })
                    .collect(),
            },
        );
        ctx
    }

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
        assert!(errors.is_empty(), "Expected no errors, got: {:?}", errors.iter().map(|e| &e.message).collect::<Vec<_>>());
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
        assert!(errors[0].message.contains("missing required input 'status'"));
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
        assert!(errors.iter().any(|e| e.message.contains("unknown input key")));
    }

    #[test]
    fn hooks_map_unknown_hook_id_fails_referential_integrity() {
        let ctx = RepoContext::new();
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
