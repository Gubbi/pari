//! [`Relay`] entity — delegates a workflow step to a shared workflow with state mapping.

use std::collections::HashMap;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::schema::{
    ids::RelayId,
    store::EntityStore,
    types::{Extensions, HooksMap, Raci, RelayStateSemantic, StateMapEntry},
    validation::{
        is_camel_case, validate_extensions, validate_hooks_map, validate_raci, ValidationError,
    },
};

#[derive(Serialize, Deserialize, JsonSchema)]
#[schemars(deny_unknown_fields)]
pub struct Relay {
    pub id: RelayId,
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
    #[serde(flatten)]
    pub extensions: Extensions,
}

pub fn validate(relay: &Relay, ctx: &EntityStore) -> Vec<ValidationError> {
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

    errors.extend(validate_extensions(&relay.extensions, "extensions"));

    errors
}

fn validate_delegates_to(relay: &Relay, ctx: &EntityStore) -> Vec<ValidationError> {
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

fn validate_state_map_keys(relay: &Relay, ctx: &EntityStore) -> Vec<ValidationError> {
    let mut errors = Vec::new();

    if let Some(states) = ctx.get_shared_workflow_states(&relay.delegates_to) {
        let valid_names: std::collections::HashSet<&str> =
            states.iter().map(String::as_str).collect();

        for key in relay.state_map.keys() {
            if !valid_names.contains(key.as_str()) {
                errors.push(ValidationError {
                    path: format!("state_map.{key}"),
                    message: format!(
                        "state_map key '{key}' does not match any state in the shared workflow"
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

fn validate_referential_integrity(relay: &Relay, ctx: &EntityStore) -> Vec<ValidationError> {
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
    use crate::schema::{
        entities::workflow::{ReviewStep, SharedWorkStepDefinition, Step, WorkflowDef},
        store::EntityStore,
        types::{
            Extensions, HookInvocation, HookInvocationValue, Raci, RelayStateSemantic,
            StateMapEntry, WorkflowSemantic, WorkflowStateEntry,
        },
    };

    fn make_shared_workflow(
        id: &str,
        state_ids: &[&str],
    ) -> crate::schema::entities::workflow::SharedWorkflow {
        let states = state_ids
            .iter()
            .enumerate()
            .map(|(i, sid)| WorkflowStateEntry {
                id: sid.to_string(),
                description: "desc".to_string(),
                semantic: if i == state_ids.len() - 1 {
                    Some(WorkflowSemantic::Complete)
                } else {
                    None
                },
            })
            .collect();
        WorkflowDef {
            id: id.into(),
            name: id.to_string(),
            description: None,
            purpose: "test".to_string(),
            accountability: Raci {
                responsible: "r".to_string(),
                accountable: "a".to_string(),
                consulted: vec![],
                informed: vec![],
            },
            steps: vec![Step::<SharedWorkStepDefinition>::Review(ReviewStep {
                id: "Gate".to_string(),
                approver: "r".to_string(),
                on_reject: "Gate".to_string(),
            })],
            states,
            hooks: None,
            guidance: None,
            extensions: Extensions::default(),
        }
    }

    fn make_ctx() -> EntityStore {
        use crate::schema::{
            entities::{hook::Hook, role::Role},
            types::Extensions,
        };
        let mut ctx = EntityStore::new();
        ctx.roles.insert(
            "eng-lead".to_string(),
            Role {
                id: "eng-lead".into(),
                name: "Engineering Lead".to_string(),
                purpose: "test".to_string(),
                traits: None,
                extensions: Extensions::default(),
            },
        );
        ctx.roles.insert(
            "pm".to_string(),
            Role {
                id: "pm".into(),
                name: "Product Manager".to_string(),
                purpose: "test".to_string(),
                traits: None,
                extensions: Extensions::default(),
            },
        );
        ctx.hooks.insert(
            "NotifySlack".to_string(),
            Hook {
                id: "NotifySlack".into(),
                name: "Notify Slack".to_string(),
                description: "test".to_string(),
                instructions: vec!["send message".to_string()],
                inputs: None,
                extensions: Extensions::default(),
            },
        );
        let sw = make_shared_workflow("LegalReview", &["Active", "Done", "Failed"]);
        ctx.shared_workflows.insert(sw.id.to_string(), sw);
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
            id: "LegalSignoff".into(),
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
            extensions: Extensions::default(),
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
            id: "legal-signoff".into(),
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

    // --- 9.3: Relay delegates_to typed vec lookup tests ---

    #[test]
    fn relay_delegates_to_found_via_typed_shared_workflow_vec() {
        let mut ctx = EntityStore::new();
        let sw = make_shared_workflow("ComplianceReview", &["Active", "Approved"]);
        ctx.shared_workflows.insert(sw.id.to_string(), sw);
        let relay = Relay {
            delegates_to: "ComplianceReview".to_string(),
            ..valid_relay()
        };
        let errors = validate_delegates_to(&relay, &ctx);
        assert!(errors.is_empty());
    }

    #[test]
    fn relay_delegates_to_not_found_in_empty_typed_vec_fails() {
        let ctx = EntityStore::new();
        let errors = validate_delegates_to(&valid_relay(), &ctx);
        assert!(!errors.is_empty());
        assert!(errors[0].message.contains("unknown shared workflow"));
    }

    // --- 8.2: Relay extensions validation tests ---

    #[test]
    fn relay_x_prefixed_extension_passes() {
        let mut map = std::collections::HashMap::new();
        map.insert("x-sla".to_string(), serde_json::json!("48h"));
        let relay = Relay {
            extensions: Extensions(map),
            ..valid_relay()
        };
        let errors = validate_structural(&relay);
        assert!(errors.is_empty());
    }

    #[test]
    fn relay_non_x_extension_key_fails() {
        let mut map = std::collections::HashMap::new();
        map.insert("sla".to_string(), serde_json::json!("48h"));
        let relay = Relay {
            extensions: Extensions(map),
            ..valid_relay()
        };
        let errors = validate_structural(&relay);
        assert!(!errors.is_empty());
        assert!(errors[0].path.contains("extensions"));
        assert!(errors[0].message.contains("x-"));
    }
}
