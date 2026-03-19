use std::collections::HashMap;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::schema::context::RepoContext;
use crate::schema::types::{HooksMap, Raci, RelayStateSemantic, StateMapEntry};
use crate::schema::validation::{is_camel_case, validate_hooks_map, validate_raci, ValidationError};

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct Relay {
    #[schemars(regex(pattern = r"^[A-Z][A-Za-z0-9]*$"))]
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub purpose: String,
    pub accountability: Option<Raci>,
    pub delegates_to: String,
    pub briefing: Option<String>,
    pub debriefing: Option<String>,
    pub state_map: HashMap<String, StateMapEntry>,
    pub hooks: Option<HooksMap>,
    pub guidance: Option<String>,
}

pub fn validate(relay: &Relay, ctx: &RepoContext) -> Vec<ValidationError> {
    let mut errors = Vec::new();

    errors.extend(validate_structural(relay));
    errors.extend(validate_delegates_to(relay, ctx));
    errors.extend(validate_state_map_keys(relay, ctx));
    errors.extend(validate_state_map_semantic(relay));
    errors.extend(validate_referential_integrity(relay, ctx));

    errors
}

fn validate_structural(relay: &Relay) -> Vec<ValidationError> {
    let mut errors = Vec::new();

    if !is_camel_case(&relay.id) {
        errors.push(ValidationError {
            path: "id".to_string(),
            message: format!("id must be CamelCase, got '{}'", relay.id),
        });
    }

    if relay.state_map.is_empty() {
        errors.push(ValidationError {
            path: "state_map".to_string(),
            message: "state_map must have at least one entry".to_string(),
        });
    }

    errors
}

fn validate_delegates_to(relay: &Relay, ctx: &RepoContext) -> Vec<ValidationError> {
    let mut errors = Vec::new();

    if !ctx.has_shared_workflow(&relay.delegates_to) {
        errors.push(ValidationError {
            path: "delegates_to".to_string(),
            message: format!(
                "unknown shared workflow '{}' in delegates_to",
                relay.delegates_to
            ),
        });
    }

    errors
}

fn validate_state_map_keys(relay: &Relay, ctx: &RepoContext) -> Vec<ValidationError> {
    let mut errors = Vec::new();

    if let Some(states) = ctx.get_shared_workflow_states(&relay.delegates_to) {
        let valid_names: std::collections::HashSet<&str> =
            states.iter().map(|s| s.as_str()).collect();

        for key in relay.state_map.keys() {
            if !valid_names.contains(key.as_str()) {
                errors.push(ValidationError {
                    path: format!("state_map.{}", key),
                    message: format!(
                        "state_map key '{}' does not match any state in the shared workflow",
                        key
                    ),
                });
            }
        }
    }
    // If shared workflow not found, delegates_to validator already reported the error

    errors
}

fn validate_state_map_semantic(relay: &Relay) -> Vec<ValidationError> {
    let mut errors = Vec::new();

    let has_complete = relay
        .state_map
        .values()
        .any(|e| e.semantic == Some(RelayStateSemantic::Complete));
    let has_non_complete = relay
        .state_map
        .values()
        .any(|e| e.semantic != Some(RelayStateSemantic::Complete));

    if !has_complete {
        errors.push(ValidationError {
            path: "state_map".to_string(),
            message: "state_map must include at least one entry with semantic: complete"
                .to_string(),
        });
    }

    if !has_non_complete {
        errors.push(ValidationError {
            path: "state_map".to_string(),
            message: "state_map must include at least one entry without semantic: complete"
                .to_string(),
        });
    }

    errors
}

fn validate_referential_integrity(relay: &Relay, ctx: &RepoContext) -> Vec<ValidationError> {
    let mut errors = Vec::new();

    if let Some(raci) = &relay.accountability {
        errors.extend(validate_raci(raci, "accountability", ctx));
    }

    if let Some(hooks) = &relay.hooks {
        errors.extend(validate_hooks_map(hooks, "hooks", ctx));
    }

    errors
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::context::{RepoContext, SharedWorkflowInfo};
    use crate::schema::types::{
        HookInvocation, HookInvocationValue, Raci, RelayStateSemantic, StateMapEntry,
    };

    fn make_ctx() -> RepoContext {
        let mut ctx = RepoContext::new();
        ctx.role_ids.insert("eng-lead".to_string());
        ctx.role_ids.insert("pm".to_string());
        ctx.hook_ids.insert("NotifySlack".to_string());
        ctx.shared_workflows.insert(
            "LegalReview".to_string(),
            SharedWorkflowInfo {
                state_ids: vec![
                    "Active".to_string(),
                    "Done".to_string(),
                    "Failed".to_string(),
                ],
            },
        );
        ctx
    }

    fn valid_state_map() -> HashMap<String, StateMapEntry> {
        let mut m = HashMap::new();
        m.insert(
            "Active".to_string(),
            StateMapEntry {
                maps_to: "InProgress".to_string(),
                semantic: None,
            },
        );
        m.insert(
            "Done".to_string(),
            StateMapEntry {
                maps_to: "Complete".to_string(),
                semantic: Some(RelayStateSemantic::Complete),
            },
        );
        m
    }

    fn valid_relay() -> Relay {
        Relay {
            id: "LegalSignoff".to_string(),
            name: "Legal Signoff".to_string(),
            description: None,
            purpose: "Ensure legal clearance".to_string(),
            accountability: None,
            delegates_to: "LegalReview".to_string(),
            briefing: None,
            debriefing: None,
            state_map: valid_state_map(),
            hooks: None,
            guidance: None,
        }
    }

    // --- 13.1: Relay structural validator tests ---

    #[test]
    fn valid_relay_passes_structural() {
        let errors = validate_structural(&valid_relay());
        assert!(errors.is_empty());
    }

    #[test]
    fn relay_kebab_id_fails() {
        let relay = Relay {
            id: "legal-signoff".to_string(),
            ..valid_relay()
        };
        let errors = validate_structural(&relay);
        assert!(!errors.is_empty());
        assert_eq!(errors[0].path, "id");
    }

    #[test]
    fn relay_empty_state_map_fails() {
        let relay = Relay {
            state_map: HashMap::new(),
            ..valid_relay()
        };
        let errors = validate_structural(&relay);
        assert!(errors.iter().any(|e| e.path == "state_map"));
    }

    // --- 13.3: Relay delegates_to referential integrity tests ---

    #[test]
    fn relay_valid_delegates_to_passes() {
        let ctx = make_ctx();
        let errors = validate_delegates_to(&valid_relay(), &ctx);
        assert!(errors.is_empty());
    }

    #[test]
    fn relay_unknown_delegates_to_fails() {
        let ctx = make_ctx();
        let relay = Relay {
            delegates_to: "UnknownWorkflow".to_string(),
            ..valid_relay()
        };
        let errors = validate_delegates_to(&relay, &ctx);
        assert!(!errors.is_empty());
        assert_eq!(errors[0].path, "delegates_to");
    }

    // --- 13.5: Relay state_map key integrity tests ---

    #[test]
    fn state_map_keys_matching_shared_workflow_passes() {
        let ctx = make_ctx();
        let errors = validate_state_map_keys(&valid_relay(), &ctx);
        assert!(errors.is_empty());
    }

    #[test]
    fn state_map_key_not_in_shared_workflow_fails() {
        let ctx = make_ctx();
        let mut state_map = valid_state_map();
        state_map.insert(
            "UnknownState".to_string(),
            StateMapEntry {
                maps_to: "Something".to_string(),
                semantic: None,
            },
        );
        let relay = Relay {
            state_map,
            ..valid_relay()
        };
        let errors = validate_state_map_keys(&relay, &ctx);
        assert!(!errors.is_empty());
        assert!(errors[0].path.contains("UnknownState"));
    }

    #[test]
    fn unmapped_shared_workflow_states_silently_pass() {
        // shared workflow has "Failed" state not covered by relay's state_map
        let ctx = make_ctx();
        let errors = validate_state_map_keys(&valid_relay(), &ctx);
        // valid_state_map has Active and Done but not Failed — that's OK
        assert!(errors.is_empty());
    }

    // --- 13.7: Relay state_map semantic constraint tests ---

    #[test]
    fn state_map_with_complete_and_non_complete_passes() {
        let errors = validate_state_map_semantic(&valid_relay());
        assert!(errors.is_empty());
    }

    #[test]
    fn state_map_missing_complete_semantic_fails() {
        let mut state_map = HashMap::new();
        state_map.insert(
            "Active".to_string(),
            StateMapEntry {
                maps_to: "InProgress".to_string(),
                semantic: None,
            },
        );
        state_map.insert(
            "Failed".to_string(),
            StateMapEntry {
                maps_to: "Failed".to_string(),
                semantic: Some(RelayStateSemantic::Failed),
            },
        );
        let relay = Relay {
            state_map,
            ..valid_relay()
        };
        let errors = validate_state_map_semantic(&relay);
        assert!(errors.iter().any(|e| e.message.contains("complete")));
    }

    #[test]
    fn state_map_all_entries_complete_fails() {
        let mut state_map = HashMap::new();
        state_map.insert(
            "Done1".to_string(),
            StateMapEntry {
                maps_to: "Complete".to_string(),
                semantic: Some(RelayStateSemantic::Complete),
            },
        );
        state_map.insert(
            "Done2".to_string(),
            StateMapEntry {
                maps_to: "AlsoComplete".to_string(),
                semantic: Some(RelayStateSemantic::Complete),
            },
        );
        let relay = Relay {
            state_map,
            ..valid_relay()
        };
        let errors = validate_state_map_semantic(&relay);
        assert!(errors.iter().any(|e| e.message.contains("without")));
    }

    // --- 13.9: Relay RACI and HooksMap referential integrity tests ---

    #[test]
    fn relay_no_raci_passes() {
        let ctx = make_ctx();
        let errors = validate_referential_integrity(&valid_relay(), &ctx);
        assert!(errors.is_empty());
    }

    #[test]
    fn relay_valid_raci_passes() {
        let ctx = make_ctx();
        let relay = Relay {
            accountability: Some(Raci {
                responsible: "eng-lead".to_string(),
                accountable: "pm".to_string(),
                consulted: vec![],
                informed: vec![],
            }),
            ..valid_relay()
        };
        let errors = validate_referential_integrity(&relay, &ctx);
        assert!(errors.is_empty());
    }

    #[test]
    fn relay_unknown_role_in_raci_fails() {
        let ctx = make_ctx();
        let relay = Relay {
            accountability: Some(Raci {
                responsible: "ghost-role".to_string(),
                accountable: "pm".to_string(),
                consulted: vec![],
                informed: vec![],
            }),
            ..valid_relay()
        };
        let errors = validate_referential_integrity(&relay, &ctx);
        assert!(!errors.is_empty());
        assert!(errors[0].path.contains("accountability.responsible"));
    }

    #[test]
    fn relay_unknown_hook_fails() {
        let ctx = make_ctx();
        let mut hooks = std::collections::HashMap::new();
        hooks.insert(
            "after".to_string(),
            HookInvocationValue::Single(HookInvocation::Bare("GhostHook".to_string())),
        );
        let relay = Relay {
            hooks: Some(hooks),
            ..valid_relay()
        };
        let errors = validate_referential_integrity(&relay, &ctx);
        assert!(!errors.is_empty());
        assert!(errors[0].message.contains("unknown hook 'GhostHook'"));
    }

    #[test]
    fn relay_valid_hook_passes() {
        let ctx = make_ctx();
        let mut hooks = std::collections::HashMap::new();
        hooks.insert(
            "after".to_string(),
            HookInvocationValue::Single(HookInvocation::Bare("NotifySlack".to_string())),
        );
        let relay = Relay {
            hooks: Some(hooks),
            ..valid_relay()
        };
        let errors = validate_referential_integrity(&relay, &ctx);
        assert!(errors.is_empty());
    }
}
