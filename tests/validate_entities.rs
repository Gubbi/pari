//! Integration tests for per-entity async validators (Task 07).

use std::collections::HashMap;

use indexmap::IndexMap;
use pari::{
    entities::{
        hook::{Hook, HookInput, TrackedHook},
        relay::{Relay, StateMapEntry, TrackedRelay},
        role::{Role, TrackedRole},
        workflow::{Step, TrackedWorkflow, Workflow},
    },
    entity::{Entity, EntityRef, WorkflowParent},
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

// ---------------------------------------------------------------------------
// Role structural tests
// ---------------------------------------------------------------------------

#[tokio::test]
async fn role_valid_passes_all_structural() {
    let role = valid_role();
    let errors = run_validations::<Role>(&role, &[], &[ValidationKind::Structural]).await;
    assert!(errors.is_empty(), "Expected no errors, got: {:?}", errors);
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

    let errors =
        run_validations::<Role>(&role, &["entity_ref"], &[ValidationKind::Structural]).await;
    assert!(!errors.is_empty(), "Expected errors for bad id");
    assert!(errors.errors.iter().any(|e| e.path == "entity_ref"));
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

    let errors = run_validations::<Role>(&role, &["name"], &[ValidationKind::Structural]).await;
    assert!(!errors.is_empty(), "Expected error for empty name");
    assert!(errors.errors.iter().any(|e| e.path == "name"));
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

    let errors = run_validations::<Role>(&role, &["purpose"], &[ValidationKind::Structural]).await;
    assert!(!errors.is_empty(), "Expected error for empty purpose");
    assert!(errors.errors.iter().any(|e| e.path == "purpose"));
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

    let errors =
        run_validations::<Role>(&role, &["extensions"], &[ValidationKind::Structural]).await;
    assert!(
        !errors.is_empty(),
        "Expected error for non-x- extension key"
    );
    assert!(errors
        .errors
        .iter()
        .any(|e| e.path.starts_with("extensions")));
}

// ---------------------------------------------------------------------------
// Hook structural tests
// ---------------------------------------------------------------------------

#[tokio::test]
async fn hook_valid_passes_structural() {
    let hook = valid_hook();
    let errors = run_validations::<Hook>(&hook, &[], &[ValidationKind::Structural]).await;
    assert!(errors.is_empty(), "Expected no errors, got: {:?}", errors);
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

    let errors = run_validations::<Hook>(&hook, &["inputs"], &[ValidationKind::Structural]).await;
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

    let errors = run_validations::<Role>(
        &role,
        &["name"], // only name
        &[ValidationKind::Structural],
    )
    .await;
    // Should pass: name is valid, purpose not checked
    assert!(
        errors.is_empty(),
        "Expected no errors when only validating 'name'"
    );
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

    let errors = run_validations::<Role>(
        &role,
        &[], // all fields
        &[ValidationKind::Structural],
    )
    .await;
    // Should fail: purpose is empty
    assert!(
        !errors.is_empty(),
        "Expected errors when running all fields with invalid purpose"
    );
    assert!(errors.errors.iter().any(|e| e.path == "purpose"));
}

// ---------------------------------------------------------------------------
// Workflow semantic tests
// ---------------------------------------------------------------------------

fn build_workflow_with_steps(steps: IndexMap<String, Step>) -> TrackedWorkflow {
    Workflow {
        entity_ref: EntityRef::new("TestWorkflow"),
        name: "Test Workflow".to_string(),
        description: None,
        purpose: "Testing".to_string(),
        raci: make_raci(),
        states: workflow_states_with_done(),
        steps,
        intercepts: None,
        guidance: None,
        extensions: Extensions::default(),
    }
    .into()
}

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
        entities::{artifact_kind::ArtifactKind, task::Task},
        types::{Artifact, TaskSemantic, TaskStateEntry},
    };

    let artifact = Artifact {
        kind: EntityRef::new("doc"),
        template: None,
    };

    let task_step_id = "WriteProposal";
    let task = Task {
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

    let errors = run_validations::<Workflow>(&wf, &["steps"], &[ValidationKind::Semantic]).await;
    // on_reject points to existing step — should pass
    assert!(
        errors
            .errors
            .iter()
            .all(|e| !e.message.contains("on_reject")),
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

    let errors = run_validations::<Workflow>(&wf, &["steps"], &[ValidationKind::Semantic]).await;
    assert!(
        errors
            .errors
            .iter()
            .any(|e| e.message.to_lowercase().contains("reviewing")),
        "Expected error about missing Reviewing state, got: {:?}",
        errors
    );
}

// ---------------------------------------------------------------------------
// Relay structural tests
// ---------------------------------------------------------------------------

fn make_relay_with_state_map(state_map: HashMap<String, StateMapEntry>) -> TrackedRelay {
    use pari::entities::{relay::Relay, workflow::ReusableWorkflow};

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

    let errors =
        run_validations::<Relay>(&relay, &["state_map"], &[ValidationKind::Structural]).await;
    assert!(!errors.is_empty(), "Expected error for empty state_map");
    assert!(errors.errors.iter().any(|e| e.path.contains("state_map")));
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

    let errors =
        run_validations::<Relay>(&relay, &["state_map"], &[ValidationKind::Structural]).await;
    assert!(
        !errors.is_empty(),
        "Expected error for non-CamelCase state key"
    );
    assert!(errors.errors.iter().any(|e| e.path.contains("state_map")));
}
