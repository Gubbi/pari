//! Shared types used across multiple entity schemas.
//!
//! Includes RACI, artifact, hook invocation, state entry, extensions, and semantic enums.

use std::collections::HashMap;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

// --- Extensions ---

/// User-defined extension fields. All keys must be prefixed with `x-`.
/// Validated at runtime by `validate_extensions`; enforced in JSON Schema via
/// `patternProperties: { "^x-": {} }` contributed when this type is flattened
/// into an entity struct.
#[derive(Default, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Extensions(pub HashMap<String, serde_json::Value>);

impl JsonSchema for Extensions {
    fn schema_name() -> String {
        "Extensions".to_string()
    }

    fn json_schema(_gen: &mut schemars::gen::SchemaGenerator) -> schemars::schema::Schema {
        use schemars::schema::{Schema, SchemaObject};
        let mut obj = SchemaObject::default();
        obj.object()
            .pattern_properties
            .insert("^x-".to_string(), Schema::Bool(true));
        Schema::Object(obj)
    }
}

// --- RACI ---

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct Raci {
    pub responsible: String,
    pub accountable: String,
    pub consulted: Vec<String>,
    pub informed: Vec<String>,
}

// --- HookInvocation and HooksMap ---

#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum HookInvocation {
    Bare(String),
    Object {
        hook: String,
        with: Option<HashMap<String, String>>,
    },
}

impl HookInvocation {
    pub fn hook_id(&self) -> &str {
        match self {
            Self::Bare(id) => id,
            Self::Object { hook, .. } => hook,
        }
    }
}

#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum HookInvocationValue {
    Single(HookInvocation),
    List(Vec<HookInvocation>),
}

impl HookInvocationValue {
    pub fn invocations(&self) -> Vec<&HookInvocation> {
        match self {
            Self::Single(inv) => vec![inv],
            Self::List(invs) => invs.iter().collect(),
        }
    }
}

pub type HooksMap = HashMap<String, HookInvocationValue>;

// Step types (WorkStep, ReviewStep, Step, WorkStepDefinition, SharedStep, etc.)
// live in entities/workflow.rs to avoid circular imports with Task/Relay entity types.

// --- State entry types ---

#[derive(Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowSemantic {
    Reviewing,
    Complete,
    Blocked,
    Failed,
}

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct WorkflowStateEntry {
    #[schemars(regex(pattern = r"^[A-Z][A-Za-z0-9]*$"))]
    pub id: String,
    pub description: String,
    pub semantic: Option<WorkflowSemantic>,
}

#[derive(Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum TaskSemantic {
    Complete,
    Blocked,
    Failed,
}

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct TaskStateEntry {
    #[schemars(regex(pattern = r"^[A-Z][A-Za-z0-9]*$"))]
    pub id: String,
    pub description: String,
    pub semantic: Option<TaskSemantic>,
}

#[derive(Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum RelayStateSemantic {
    Complete,
    Blocked,
    Failed,
}

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct StateMapEntry {
    pub maps_to: String,
    pub semantic: Option<RelayStateSemantic>,
}

// --- Artifact ---

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct Artifact {
    pub name: String,
    pub template: Option<String>,
}

// --- Tests ---

#[cfg(test)]
mod tests {
    use super::*;

    // --- 6.1: Raci struct tests ---

    #[test]
    fn raci_with_empty_lists_is_valid() {
        let r = Raci {
            responsible: "eng-lead".to_string(),
            accountable: "pm".to_string(),
            consulted: vec![],
            informed: vec![],
        };
        assert_eq!(r.responsible, "eng-lead");
        assert_eq!(r.accountable, "pm");
        assert!(r.consulted.is_empty());
        assert!(r.informed.is_empty());
    }

    #[test]
    fn raci_with_lists() {
        let r = Raci {
            responsible: "eng-lead".to_string(),
            accountable: "pm".to_string(),
            consulted: vec!["designer".to_string()],
            informed: vec!["sre-lead".to_string()],
        };
        assert_eq!(r.consulted.len(), 1);
        assert_eq!(r.informed.len(), 1);
    }

    // --- 6.3: HookInvocation and HooksMap tests ---

    #[test]
    fn hook_invocation_bare_string() {
        let inv = HookInvocation::Bare("NotifySlack".to_string());
        assert_eq!(inv.hook_id(), "NotifySlack");
    }

    #[test]
    fn hook_invocation_object_with_inputs() {
        let mut with = HashMap::new();
        with.insert("status".to_string(), "Done".to_string());
        let inv = HookInvocation::Object {
            hook: "UpdateJiraStatus".to_string(),
            with: Some(with),
        };
        assert_eq!(inv.hook_id(), "UpdateJiraStatus");
    }

    #[test]
    fn hook_invocation_object_without_with() {
        let inv = HookInvocation::Object {
            hook: "UpdateJiraStatus".to_string(),
            with: None,
        };
        assert_eq!(inv.hook_id(), "UpdateJiraStatus");
        if let HookInvocation::Object { with, .. } = &inv {
            assert!(with.is_none());
        }
    }

    #[test]
    fn hooks_map_single_invocation() {
        let mut map: HooksMap = HashMap::new();
        map.insert(
            "after".to_string(),
            HookInvocationValue::Single(HookInvocation::Bare("NotifySlack".to_string())),
        );
        let val = map.get("after").unwrap();
        assert_eq!(val.invocations().len(), 1);
    }

    #[test]
    fn hooks_map_list_invocation() {
        let mut map: HooksMap = HashMap::new();
        map.insert(
            "after".to_string(),
            HookInvocationValue::List(vec![
                HookInvocation::Bare("NotifySlack".to_string()),
                HookInvocation::Object {
                    hook: "UpdateJiraStatus".to_string(),
                    with: None,
                },
            ]),
        );
        let val = map.get("after").unwrap();
        assert_eq!(val.invocations().len(), 2);
    }

    // --- 8.1: State entry type tests ---

    #[test]
    fn workflow_state_entry_with_reviewing_semantic() {
        let e = WorkflowStateEntry {
            id: "UnderReview".to_string(),
            description: "Awaiting gate decision".to_string(),
            semantic: Some(WorkflowSemantic::Reviewing),
        };
        assert_eq!(e.semantic, Some(WorkflowSemantic::Reviewing));
    }

    #[test]
    fn workflow_state_entry_without_semantic() {
        let e = WorkflowStateEntry {
            id: "Active".to_string(),
            description: "Work underway".to_string(),
            semantic: None,
        };
        assert!(e.semantic.is_none());
    }

    #[test]
    fn workflow_state_entry_complete_semantic() {
        let e = WorkflowStateEntry {
            id: "Done".to_string(),
            description: "Completed".to_string(),
            semantic: Some(WorkflowSemantic::Complete),
        };
        assert_eq!(e.semantic, Some(WorkflowSemantic::Complete));
    }

    #[test]
    fn task_state_entry_with_complete_semantic() {
        let e = TaskStateEntry {
            id: "Done".to_string(),
            description: "Completed".to_string(),
            semantic: Some(TaskSemantic::Complete),
        };
        assert_eq!(e.semantic, Some(TaskSemantic::Complete));
    }

    #[test]
    fn task_state_entry_without_semantic() {
        let e = TaskStateEntry {
            id: "Draft".to_string(),
            description: "Being written".to_string(),
            semantic: None,
        };
        assert!(e.semantic.is_none());
    }

    #[test]
    fn task_state_entry_reviewing_not_available() {
        // TaskSemantic enum does not have Reviewing variant — enforced at type level
        // This test documents the constraint via exhaustive match
        let e = TaskStateEntry {
            id: "Done".to_string(),
            description: "Completed".to_string(),
            semantic: Some(TaskSemantic::Complete),
        };
        match e.semantic.unwrap() {
            TaskSemantic::Complete => {}
            TaskSemantic::Blocked => {}
            TaskSemantic::Failed => {} // No Reviewing variant — compiler enforces this
        }
    }

    #[test]
    fn state_map_entry_with_semantic() {
        let e = StateMapEntry {
            maps_to: "Complete".to_string(),
            semantic: Some(RelayStateSemantic::Complete),
        };
        assert_eq!(e.maps_to, "Complete");
        assert_eq!(e.semantic, Some(RelayStateSemantic::Complete));
    }

    #[test]
    fn state_map_entry_without_semantic() {
        let e = StateMapEntry {
            maps_to: "Active".to_string(),
            semantic: None,
        };
        assert!(e.semantic.is_none());
    }

    // --- Extensions tests ---

    #[test]
    fn extensions_serde_round_trip_x_keys() {
        let json = r#"{"x-team": "platform", "x-sla": "24h"}"#;
        let ext: Extensions = serde_json::from_str(json).unwrap();
        assert_eq!(ext.0.len(), 2);
        assert_eq!(ext.0["x-team"], serde_json::json!("platform"));
        let back = serde_json::to_string(&ext).unwrap();
        let re: serde_json::Value = serde_json::from_str(&back).unwrap();
        assert_eq!(re["x-team"], "platform");
    }

    #[test]
    fn extensions_empty_round_trip() {
        let ext: Extensions = serde_json::from_str("{}").unwrap();
        assert!(ext.0.is_empty());
    }

    #[test]
    fn extensions_schema_has_pattern_properties() {
        use schemars::schema_for;

        // Define a minimal struct that flattens Extensions to test schema output.
        #[derive(schemars::JsonSchema)]
        #[allow(dead_code)]
        struct WithExt {
            id: String,
            #[schemars(flatten)]
            extensions: Extensions,
        }

        let schema = schema_for!(WithExt);
        let json = serde_json::to_value(&schema).unwrap();

        // schemars 0.8 DOES propagate patternProperties from flattened custom JsonSchema impls
        // natively — no xtask post-processing required.
        let pattern_props = &json["patternProperties"];
        assert!(
            !pattern_props.is_null(),
            "Expected patternProperties in schema root, got: {}",
            json
        );
        assert_eq!(pattern_props["^x-"], serde_json::json!(true));
    }

    #[test]
    fn deny_unknown_fields_with_flatten_does_not_emit_additional_properties() {
        use schemars::schema_for;

        // schemars 0.8 known limitation: when a struct has a `#[serde(flatten)]` field,
        // `deny_unknown_fields` does NOT emit `additionalProperties: false` — schemars
        // treats additional key space as owned by the flattened schema.
        // Entity schemas require xtask post-processing to add `additionalProperties: false`.
        #[derive(schemars::JsonSchema)]
        #[schemars(deny_unknown_fields)]
        #[allow(dead_code)]
        struct WithExtStrict {
            id: String,
            #[schemars(flatten)]
            extensions: Extensions,
        }

        let schema = schema_for!(WithExtStrict);
        let json = serde_json::to_value(&schema).unwrap();

        // patternProperties IS present (from Extensions flatten)
        assert_eq!(json["patternProperties"]["^x-"], serde_json::json!(true));
        // additionalProperties: false is NOT emitted by schemars 0.8 when flatten is present
        assert!(
            json["additionalProperties"].is_null(),
            "schemars 0.8 behavior changed: now emitting additionalProperties with flatten. \
             xtask post-processing may no longer be needed. schema: {}",
            json
        );
    }
}
