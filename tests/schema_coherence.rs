/// Schema coherence tests — verify that schemars annotations on Rust types produce
/// JSON schemas that match the structural constraints from the original hand-written schemas.
use pari::schema::entities::hook::Hook;
use pari::schema::entities::relay::Relay;
use pari::schema::entities::role::Role;
use pari::schema::entities::task::Task;
use pari::schema::entities::team::Team;
use pari::schema::entities::workflow::Workflow;
use pari::schema::types::{
    Artifact, HookInvocation, HookInvocationValue, Raci, RelayStateSemantic, StateMapEntry, Step,
    TaskSemantic, TaskStateEntry, WorkflowSemantic, WorkflowStateEntry,
};
use schemars::{schema_for, JsonSchema};
use serde_json::Value;

fn schema_json<T: JsonSchema>() -> Value {
    serde_json::to_value(schema_for!(T)).unwrap()
}

fn required_fields(schema: &Value) -> Vec<String> {
    schema["required"]
        .as_array()
        .unwrap_or(&vec![])
        .iter()
        .filter_map(|v| v.as_str().map(String::from))
        .collect()
}

fn enum_values(schema: &Value) -> Vec<String> {
    let direct = schema["enum"].as_array();
    if let Some(arr) = direct {
        return arr.iter().filter_map(|v| v.as_str().map(String::from)).collect();
    }
    schema["anyOf"]
        .as_array()
        .unwrap_or(&vec![])
        .iter()
        .flat_map(|v| {
            v["enum"]
                .as_array()
                .unwrap_or(&vec![])
                .iter()
                .filter_map(|e| e.as_str().map(String::from))
                .collect::<Vec<_>>()
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Raci
// ---------------------------------------------------------------------------

#[test]
fn raci_all_fields_required() {
    let schema = schema_json::<Raci>();
    let required = required_fields(&schema);
    assert!(required.contains(&"responsible".to_string()));
    assert!(required.contains(&"accountable".to_string()));
    assert!(required.contains(&"consulted".to_string()));
    assert!(required.contains(&"informed".to_string()));
}

#[test]
fn raci_consulted_and_informed_are_arrays() {
    let schema = schema_json::<Raci>();
    assert_eq!(schema["properties"]["consulted"]["type"], "array");
    assert_eq!(schema["properties"]["informed"]["type"], "array");
}

// ---------------------------------------------------------------------------
// HookInvocation — anyOf: bare string | object
// ---------------------------------------------------------------------------

#[test]
fn hook_invocation_is_any_of() {
    let schema = schema_json::<HookInvocation>();
    assert!(schema["anyOf"].is_array(), "HookInvocation should generate anyOf");
}

#[test]
fn hook_invocation_has_string_variant() {
    let schema = schema_json::<HookInvocation>();
    let variants = schema["anyOf"].as_array().unwrap();
    let has_string = variants.iter().any(|v| v["type"] == "string");
    assert!(has_string, "HookInvocation should have a string variant");
}

// ---------------------------------------------------------------------------
// HookInvocationValue — anyOf: single | list
// ---------------------------------------------------------------------------

#[test]
fn hook_invocation_value_is_any_of() {
    let schema = schema_json::<HookInvocationValue>();
    assert!(schema["anyOf"].is_array(), "HookInvocationValue should generate anyOf");
}

// ---------------------------------------------------------------------------
// Step — anyOf: WorkStep | ReviewStep
// ---------------------------------------------------------------------------

#[test]
fn step_is_any_of() {
    let schema = schema_json::<Step>();
    assert!(schema["anyOf"].is_array(), "Step should generate anyOf");
}

#[test]
fn step_has_two_variants() {
    let schema = schema_json::<Step>();
    let variants = schema["anyOf"].as_array().unwrap();
    assert_eq!(variants.len(), 2, "Step should have exactly two variants");
}

// ---------------------------------------------------------------------------
// Semantic enums — snake_case string values
// ---------------------------------------------------------------------------

#[test]
fn workflow_semantic_has_all_values() {
    let schema = schema_json::<WorkflowSemantic>();
    let values = enum_values(&schema);
    assert!(values.contains(&"reviewing".to_string()));
    assert!(values.contains(&"complete".to_string()));
    assert!(values.contains(&"blocked".to_string()));
    assert!(values.contains(&"failed".to_string()));
}

#[test]
fn task_semantic_has_no_reviewing() {
    let schema = schema_json::<TaskSemantic>();
    let values = enum_values(&schema);
    assert!(!values.contains(&"reviewing".to_string()));
    assert!(values.contains(&"complete".to_string()));
    assert!(values.contains(&"blocked".to_string()));
    assert!(values.contains(&"failed".to_string()));
}

#[test]
fn relay_state_semantic_has_all_values() {
    let schema = schema_json::<RelayStateSemantic>();
    let values = enum_values(&schema);
    assert!(values.contains(&"complete".to_string()));
    assert!(values.contains(&"blocked".to_string()));
    assert!(values.contains(&"failed".to_string()));
}

// ---------------------------------------------------------------------------
// WorkflowStateEntry / TaskStateEntry / StateMapEntry
// ---------------------------------------------------------------------------

#[test]
fn workflow_state_entry_required_fields() {
    let schema = schema_json::<WorkflowStateEntry>();
    let required = required_fields(&schema);
    assert!(required.contains(&"id".to_string()));
    assert!(required.contains(&"description".to_string()));
}

#[test]
fn task_state_entry_required_fields() {
    let schema = schema_json::<TaskStateEntry>();
    let required = required_fields(&schema);
    assert!(required.contains(&"id".to_string()));
    assert!(required.contains(&"description".to_string()));
}

#[test]
fn state_map_entry_maps_to_required() {
    let schema = schema_json::<StateMapEntry>();
    let required = required_fields(&schema);
    assert!(required.contains(&"maps_to".to_string()));
}

// ---------------------------------------------------------------------------
// Artifact
// ---------------------------------------------------------------------------

#[test]
fn artifact_name_required() {
    let schema = schema_json::<Artifact>();
    let required = required_fields(&schema);
    assert!(required.contains(&"name".to_string()));
}

// ---------------------------------------------------------------------------
// Role
// ---------------------------------------------------------------------------

#[test]
fn role_id_has_kebab_pattern() {
    let schema = schema_json::<Role>();
    let pattern = schema["properties"]["id"]["pattern"].as_str().unwrap();
    assert_eq!(pattern, r"^[a-z][a-z0-9-]*$");
}

// ---------------------------------------------------------------------------
// Hook
// ---------------------------------------------------------------------------

#[test]
fn hook_id_has_camel_pattern() {
    let schema = schema_json::<Hook>();
    let pattern = schema["properties"]["id"]["pattern"].as_str().unwrap();
    assert_eq!(pattern, r"^[A-Z][A-Za-z0-9]*$");
}

#[test]
fn hook_instructions_has_min_items_1() {
    let schema = schema_json::<Hook>();
    assert_eq!(schema["properties"]["instructions"]["minItems"], 1);
}

// ---------------------------------------------------------------------------
// Team
// ---------------------------------------------------------------------------

#[test]
fn team_id_has_kebab_pattern() {
    let schema = schema_json::<Team>();
    let pattern = schema["properties"]["id"]["pattern"].as_str().unwrap();
    assert_eq!(pattern, r"^[a-z][a-z0-9-]*$");
}

// ---------------------------------------------------------------------------
// Workflow
// ---------------------------------------------------------------------------

#[test]
fn workflow_id_has_camel_pattern() {
    let schema = schema_json::<Workflow>();
    let pattern = schema["properties"]["id"]["pattern"].as_str().unwrap();
    assert_eq!(pattern, r"^[A-Z][A-Za-z0-9]*$");
}

#[test]
fn workflow_steps_has_min_items_1() {
    let schema = schema_json::<Workflow>();
    assert_eq!(schema["properties"]["steps"]["minItems"], 1);
}

#[test]
fn workflow_states_has_min_items_2() {
    let schema = schema_json::<Workflow>();
    assert_eq!(schema["properties"]["states"]["minItems"], 2);
}

// ---------------------------------------------------------------------------
// Task
// ---------------------------------------------------------------------------

#[test]
fn task_id_has_camel_pattern() {
    let schema = schema_json::<Task>();
    let pattern = schema["properties"]["id"]["pattern"].as_str().unwrap();
    assert_eq!(pattern, r"^[A-Z][A-Za-z0-9]*$");
}

#[test]
fn task_instructions_has_min_items_1() {
    let schema = schema_json::<Task>();
    assert_eq!(schema["properties"]["instructions"]["minItems"], 1);
}

#[test]
fn task_criteria_has_min_items_1() {
    let schema = schema_json::<Task>();
    assert_eq!(schema["properties"]["criteria"]["minItems"], 1);
}

#[test]
fn task_states_has_min_items_2() {
    let schema = schema_json::<Task>();
    assert_eq!(schema["properties"]["states"]["minItems"], 2);
}

// ---------------------------------------------------------------------------
// Relay
// ---------------------------------------------------------------------------

#[test]
fn relay_id_has_camel_pattern() {
    let schema = schema_json::<Relay>();
    let pattern = schema["properties"]["id"]["pattern"].as_str().unwrap();
    assert_eq!(pattern, r"^[A-Z][A-Za-z0-9]*$");
}
