//! Integration tests for per-entity async validators (Task 07).

use std::collections::HashMap;

use indexmap::IndexMap;
use pari::{
    entities::{
        hook::{Hook, HookInput, TrackedHook},
        relay::{StateMapEntry, TrackedRelay},
        role::{Role, TrackedRole},
        workflow::{Step, TrackedWorkflow, Workflow},
    },
    entity::{EntityRef, WorkflowParent},
    error::{primitive::PrimitiveError, ActivityError},
    types::{Extensions, Raci, WorkflowSemantic, WorkflowStateEntry},
    validation::{run_validations, ValidationKind},
};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn role_ref(id: &str) -> EntityRef<pari::entities::role::Role> {
    EntityRef::new(id)
}

fn workflow_parent(id: &str) -> WorkflowParent {
    WorkflowParent::Workflow(EntityRef::new(id))
}

fn make_raci() -> Raci {
    Raci {
        responsible: vec![role_ref("eng-lead")],
        accountable: role_ref("pm"),
        consulted: None,
        informed: None,
    }
}

fn workflow_states_with_done() -> Vec<WorkflowStateEntry> {
    vec![
        WorkflowStateEntry {
            id: "Active".to_string(),
            description: "In progress".to_string(),
            semantic: None,
        },
        WorkflowStateEntry {
            id: "Done".to_string(),
            description: "Complete".to_string(),
            semantic: Some(WorkflowSemantic::Done),
        },
    ]
}

fn workflow_states_with_reviewing() -> Vec<WorkflowStateEntry> {
    vec![
        WorkflowStateEntry {
            id: "Active".to_string(),
            description: "In progress".to_string(),
            semantic: None,
        },
        WorkflowStateEntry {
            id: "Reviewing".to_string(),
            description: "Under review".to_string(),
            semantic: Some(WorkflowSemantic::Reviewing),
        },
        WorkflowStateEntry {
            id: "Done".to_string(),
            description: "Complete".to_string(),
            semantic: Some(WorkflowSemantic::Done),
        },
    ]
}

fn valid_role() -> TrackedRole {
    Role {
        entity_ref: EntityRef::new("eng-lead"),
        name: "Engineering Lead".to_string(),
        description: None,
        purpose: "Lead engineering efforts".to_string(),
        traits: None,
        extensions: Extensions::default(),
    }
    .into()
}

fn valid_hook() -> TrackedHook {
    Hook {
        entity_ref: EntityRef::new("send-notification"),
        name: "Send Notification".to_string(),
        description: None,
        instructions: vec!["Step one".to_string()],
        inputs: Some(vec![HookInput {
            name: "recipient".to_string(),
            description: Some("Who to notify".to_string()),
            required: true,
        }]),
        extensions: Extensions::default(),
    }
    .into()
}

/// Extracts the field-level error map from a `ValidationFailed` result, or
/// returns an empty map for `Ok(())`.  Panics on any other error variant.
fn extract_validation_errors(
    result: Result<(), ActivityError>,
) -> HashMap<String, Vec<PrimitiveError>> {
    match result {
        Ok(()) => HashMap::new(),
        Err(ActivityError::ValidationFailed { cause, .. }) => {
            if let PrimitiveError::FieldValidationError { errors, .. } = cause {
                errors
            } else {
                panic!("Expected FieldValidationError inside ValidationFailed, got: {cause:?}");
            }
        }
        Err(other) => panic!("Expected ValidationFailed, got: {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// Role structural tests
// ---------------------------------------------------------------------------

#[tokio::test]
async fn role_valid_passes_all_structural() {
    run_validations::<Role>(&valid_role(), &[], &[ValidationKind::Structural])
        .await
        .expect("no validation errors for valid role");
}

#[tokio::test]
async fn role_invalid_id_fails_structural() {
    let role: TrackedRole = Role {
        entity_ref: EntityRef::new("EngLead"), // not kebab-case
        name: "Engineering Lead".to_string(),
        description: None,
        purpose: "Lead engineering".to_string(),
        traits: None,
        extensions: Extensions::default(),
    }
    .into();

    let result =
        run_validations::<Role>(&role, &["entity_ref"], &[ValidationKind::Structural]).await;
    let errors = extract_validation_errors(result);
    assert!(!errors.is_empty(), "Expected errors for bad id");
    assert!(
        errors.contains_key("entity_ref"),
        "Expected error on entity_ref"
    );
}

#[tokio::test]
async fn role_empty_name_fails_structural() {
    let role: TrackedRole = Role {
        entity_ref: EntityRef::new("eng-lead"),
        name: "".to_string(),
        description: None,
        purpose: "Lead engineering".to_string(),
        traits: None,
        extensions: Extensions::default(),
    }
    .into();

    let result = run_validations::<Role>(&role, &["name"], &[ValidationKind::Structural]).await;
    let errors = extract_validation_errors(result);
    assert!(!errors.is_empty(), "Expected error for empty name");
    assert!(errors.contains_key("name"), "Expected error on name");
}

#[tokio::test]
async fn role_empty_purpose_fails_structural() {
    let role: TrackedRole = Role {
        entity_ref: EntityRef::new("eng-lead"),
        name: "Engineering Lead".to_string(),
        description: None,
        purpose: "  ".to_string(), // whitespace-only
        traits: None,
        extensions: Extensions::default(),
    }
    .into();

    let result = run_validations::<Role>(&role, &["purpose"], &[ValidationKind::Structural]).await;
    let errors = extract_validation_errors(result);
    assert!(!errors.is_empty(), "Expected error for empty purpose");
    assert!(errors.contains_key("purpose"), "Expected error on purpose");
}

#[tokio::test]
async fn role_non_x_extension_fails_structural() {
    let mut ext = Extensions::new();
    ext.insert("owner".to_string(), serde_json::json!("alice"));

    let role: TrackedRole = Role {
        entity_ref: EntityRef::new("eng-lead"),
        name: "Engineering Lead".to_string(),
        description: None,
        purpose: "Lead engineering".to_string(),
        traits: None,
        extensions: ext,
    }
    .into();

    let result =
        run_validations::<Role>(&role, &["extensions"], &[ValidationKind::Structural]).await;
    let errors = extract_validation_errors(result);
    assert!(
        !errors.is_empty(),
        "Expected error for non-x- extension key"
    );
    assert!(
        errors.keys().any(|k| k.starts_with("extensions")),
        "Expected error on extensions"
    );
}

// ---------------------------------------------------------------------------
// Hook structural tests
// ---------------------------------------------------------------------------

#[tokio::test]
async fn hook_valid_passes_structural() {
    run_validations::<Hook>(&valid_hook(), &[], &[ValidationKind::Structural])
        .await
        .expect("no validation errors for valid hook");
}

#[tokio::test]
async fn hook_duplicate_input_names_fails_structural() {
    let hook: TrackedHook = Hook {
        entity_ref: EntityRef::new("my-hook"),
        name: "My Hook".to_string(),
        description: None,
        instructions: vec!["do it".to_string()],
        inputs: Some(vec![
            HookInput {
                name: "param1".to_string(),
                description: None,
                required: false,
            },
            HookInput {
                name: "param1".to_string(),
                description: None,
                required: true,
            }, // duplicate
        ]),
        extensions: Extensions::default(),
    }
    .into();

    let result = run_validations::<Hook>(&hook, &["inputs"], &[ValidationKind::Structural]).await;
    let errors = extract_validation_errors(result);
    assert!(
        !errors.is_empty(),
        "Expected error for duplicate input names"
    );
}

// ---------------------------------------------------------------------------
// run_validations control tests
// ---------------------------------------------------------------------------

#[tokio::test]
async fn run_validations_only_runs_requested_fields() {
    // Role with bad purpose but valid name — only validate name, should pass
    let role: TrackedRole = Role {
        entity_ref: EntityRef::new("eng-lead"),
        name: "Engineering Lead".to_string(),
        description: None,
        purpose: "".to_string(), // invalid
        traits: None,
        extensions: Extensions::default(),
    }
    .into();

    run_validations::<Role>(&role, &["name"], &[ValidationKind::Structural])
        .await
        .expect("should pass when only validating 'name'");
}

#[tokio::test]
async fn run_validations_empty_fields_runs_all() {
    // Role with bad purpose — empty fields means run all
    let role: TrackedRole = Role {
        entity_ref: EntityRef::new("eng-lead"),
        name: "Engineering Lead".to_string(),
        description: None,
        purpose: "".to_string(), // invalid
        traits: None,
        extensions: Extensions::default(),
    }
    .into();

    let result = run_validations::<Role>(&role, &[], &[ValidationKind::Structural]).await;
    let errors = extract_validation_errors(result);
    assert!(
        !errors.is_empty(),
        "Expected errors when running all fields with invalid purpose"
    );
    assert!(errors.contains_key("purpose"), "Expected error on purpose");
}

#[tokio::test]
async fn run_validations_unknown_field_returns_err() {
    let result = run_validations::<Role>(
        &valid_role(),
        &["nonexistent_field"],
        &[ValidationKind::Structural],
    )
    .await;
    assert!(
        matches!(result, Err(ActivityError::PariInvariantViolation { .. })),
        "Expected PariInvariantViolation for unknown field, got: {result:?}",
    );
}

// ---------------------------------------------------------------------------
// Workflow semantic tests
// ---------------------------------------------------------------------------

fn build_workflow_with_reviewing_steps(steps: IndexMap<String, Step>) -> TrackedWorkflow {
    Workflow {
        entity_ref: EntityRef::new("TestWorkflow"),
        name: "Test Workflow".to_string(),
        description: None,
        purpose: "Testing".to_string(),
        raci: make_raci(),
        states: workflow_states_with_reviewing(),
        steps,
        intercepts: None,
        guidance: None,
        extensions: Extensions::default(),
    }
    .into()
}

#[tokio::test]
async fn workflow_on_reject_valid_passes_when_target_exists() {
    use pari::{
        entities::task::Task,
        types::{Artifact, TaskSemantic, TaskStateEntry},
    };

    let artifact = Artifact {
        kind: EntityRef::new("doc"),
        template: None,
    };

    let task_step_id = "WriteProposal";
    let _task = Task {
        entity_ref: EntityRef::with_parent(task_step_id, workflow_parent("TestWorkflow")),
        name: "Write Proposal".to_string(),
        description: None,
        purpose: "Produce proposal".to_string(),
        instructions: vec!["Write it".to_string()],
        criteria: vec!["Done".to_string()],
        raci: None,
        artifact,
        states: vec![
            TaskStateEntry {
                id: "Active".to_string(),
                description: "In progress".to_string(),
                semantic: None,
            },
            TaskStateEntry {
                id: "Done".to_string(),
                description: "Complete".to_string(),
                semantic: Some(TaskSemantic::Done),
            },
        ],
        intercepts: None,
        guidance: None,
        extensions: Extensions::default(),
    };

    let mut steps = IndexMap::new();
    steps.insert(
        task_step_id.to_string(),
        Step::Task {
            entity_ref: EntityRef::with_parent(task_step_id, workflow_parent("TestWorkflow")),
            depends_on: None,
        },
    );
    // Review step with valid on_reject target
    steps.insert(
        "ReviewProposal".to_string(),
        Step::Review {
            approver: vec![role_ref("pm")],
            on_reject: task_step_id.to_string(), // valid target
        },
    );

    let wf = build_workflow_with_reviewing_steps(steps);

    let result = run_validations::<Workflow>(&wf, &["steps"], &[ValidationKind::Semantic]).await;
    let errors = extract_validation_errors(result);
    // on_reject points to an existing step — no InvalidOnRejectTarget errors
    assert!(
        errors
            .values()
            .flat_map(|v| v.iter())
            .all(|e| !matches!(e, PrimitiveError::InvalidOnRejectTarget { .. })),
        "on_reject validation should pass when target exists"
    );
}

#[tokio::test]
async fn workflow_reviewing_state_required_when_review_steps_present() {
    // Workflow has a Review step but states do NOT include Reviewing semantic
    let mut steps = IndexMap::new();
    steps.insert(
        "ReviewProposal".to_string(),
        Step::Review {
            approver: vec![role_ref("pm")],
            on_reject: "some-step".to_string(),
        },
    );

    // Build workflow WITHOUT reviewing state
    let wf: TrackedWorkflow = Workflow {
        entity_ref: EntityRef::new("TestWorkflow"),
        name: "Test Workflow".to_string(),
        description: None,
        purpose: "Testing".to_string(),
        raci: make_raci(),
        states: workflow_states_with_done(), // no Reviewing state
        steps,
        intercepts: None,
        guidance: None,
        extensions: Extensions::default(),
    }
    .into();

    let result = run_validations::<Workflow>(&wf, &["steps"], &[ValidationKind::Semantic]).await;
    let errors = extract_validation_errors(result);
    assert!(
        errors
            .values()
            .flat_map(|v| v.iter())
            .any(|e| matches!(e, PrimitiveError::WorkflowGraphInconsistency { .. })),
        "Expected WorkflowGraphInconsistency error for missing Reviewing state, got: {errors:?}",
    );
}

// ---------------------------------------------------------------------------
// Relay structural tests
// ---------------------------------------------------------------------------

fn make_relay_with_state_map(state_map: HashMap<String, StateMapEntry>) -> TrackedRelay {
    use pari::entities::relay::Relay;

    Relay {
        entity_ref: EntityRef::with_parent("DelegateTask", workflow_parent("TestWorkflow")),
        name: "Delegate Task".to_string(),
        description: None,
        purpose: "Delegate work".to_string(),
        raci: None,
        delegates_to: EntityRef::new("SomeWorkflow"),
        briefing: None,
        debriefing: None,
        state_map,
        intercepts: None,
        guidance: None,
        extensions: Extensions::default(),
    }
    .into()
}

#[tokio::test]
async fn relay_empty_state_map_fails_structural() {
    use pari::entities::relay::Relay;

    let relay = make_relay_with_state_map(HashMap::new());

    let result =
        run_validations::<Relay>(&relay, &["state_map"], &[ValidationKind::Structural]).await;
    let errors = extract_validation_errors(result);
    assert!(!errors.is_empty(), "Expected error for empty state_map");
    assert!(
        errors.keys().any(|k| k.contains("state_map")),
        "Expected error on state_map"
    );
}

#[tokio::test]
async fn relay_non_camel_case_state_key_fails_structural() {
    use pari::entities::relay::{Relay, StateMapEntry};

    let mut state_map = HashMap::new();
    state_map.insert(
        "active-state".to_string(), // not CamelCase
        StateMapEntry {
            maps_to: "Active".to_string(),
            description: None,
            semantic: None,
        },
    );

    let relay = make_relay_with_state_map(state_map);

    let result =
        run_validations::<Relay>(&relay, &["state_map"], &[ValidationKind::Structural]).await;
    let errors = extract_validation_errors(result);
    assert!(
        !errors.is_empty(),
        "Expected error for non-CamelCase state key"
    );
    assert!(
        errors.keys().any(|k| k.contains("state_map")),
        "Expected error on state_map"
    );
}
