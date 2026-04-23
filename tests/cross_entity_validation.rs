//! Cross-entity validation tests (Task 12 / TDD).
//!
//! Each test exercises the `CrossEntity` validation kind, which needs an
//! active `EntityServer` session so that `EntityClient::has_ref` can reach
//! the store.  All tests therefore run inside `EntityServer::with(...)`.
//!
//! Tests are written before the implementation in `cross_entity/common.rs`
//! (Task 13) so they initially fail on the stubs.

use indexmap::IndexMap;
use pari::{
    entities::{
        artifact_kind::{ArtifactKind, TrackedArtifactKind},
        relay::{Relay, StateMapEntry, TrackedRelay},
        role::{Role, TrackedRole},
        task::{Task, TrackedTask},
        workflow::{ReusableWorkflow, Step, TrackedWorkflow, Workflow},
    },
    entity::{AnyEntityRef, EntityRef, TrackedEntity, WorkflowParent},
    error::{primitive::PrimitiveError, ActivityError},
    store::EntityServer,
    substrate::InMemorySubstrate,
    types::{
        Artifact, Extensions, Raci, TaskSemantic, TaskStateEntry, WorkflowSemantic,
        WorkflowStateEntry,
    },
    validation::{run_validations, ValidationKind},
    workspace::EntityClient,
};

// ---------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------

fn role(id: &str) -> TrackedRole {
    Role {
        entity_ref: EntityRef::new(id),
        name: id.to_string(),
        description: None,
        purpose: "Purpose".to_string(),
        traits: None,
        extensions: Extensions::default(),
    }
    .into()
}

fn artifact_kind(id: &str) -> TrackedArtifactKind {
    ArtifactKind {
        entity_ref: EntityRef::new(id),
        name: id.to_string(),
        description: None,
        service: "storage".to_string(),
        access: None,
        guidance: None,
        extensions: Extensions::default(),
    }
    .into()
}

fn raci(role_id: &str) -> Raci {
    Raci {
        responsible: vec![EntityRef::new(role_id)],
        accountable: EntityRef::new(role_id),
        consulted: None,
        informed: None,
    }
}

fn workflow_parent(id: &str) -> WorkflowParent {
    WorkflowParent::Workflow(EntityRef::new(id))
}

fn workflow_states() -> Vec<WorkflowStateEntry> {
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

fn task_states() -> Vec<TaskStateEntry> {
    vec![
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
    ]
}

fn task(step_id: &str, wf_id: &str, artifact_kind_id: &str) -> TrackedTask {
    Task {
        entity_ref: EntityRef::with_parent(step_id, workflow_parent(wf_id)),
        name: step_id.to_string(),
        description: None,
        purpose: "Do something".to_string(),
        instructions: vec!["Do it".to_string()],
        criteria: vec!["Done".to_string()],
        raci: None,
        artifact: Artifact {
            kind: EntityRef::new(artifact_kind_id),
            template: None,
        },
        states: task_states(),
        intercepts: None,
        guidance: None,
        extensions: Extensions::default(),
    }
    .into()
}

fn simple_workflow(wf_id: &str, role_id: &str, task_step_id: &str) -> TrackedWorkflow {
    let mut steps = IndexMap::new();
    steps.insert(
        task_step_id.to_string(),
        Step::Task {
            entity_ref: EntityRef::with_parent(task_step_id, workflow_parent(wf_id)),
            depends_on: None,
        },
    );
    Workflow {
        entity_ref: EntityRef::new(wf_id),
        name: wf_id.to_string(),
        description: None,
        purpose: "Testing".to_string(),
        raci: raci(role_id),
        states: workflow_states(),
        steps,
        intercepts: None,
        guidance: None,
        extensions: Extensions::default(),
    }
    .into()
}

fn relay(step_id: &str, wf_id: &str, delegates_to_id: &str) -> TrackedRelay {
    let mut state_map = std::collections::HashMap::new();
    state_map.insert(
        "Done".to_string(),
        StateMapEntry {
            maps_to: "Done".to_string(),
            description: None,
            semantic: None,
        },
    );
    Relay {
        entity_ref: EntityRef::with_parent(step_id, workflow_parent(wf_id)),
        name: step_id.to_string(),
        description: None,
        purpose: "Delegate".to_string(),
        raci: None,
        delegates_to: EntityRef::new(delegates_to_id),
        briefing: None,
        debriefing: None,
        state_map,
        intercepts: None,
        guidance: None,
        extensions: Extensions::default(),
    }
    .into()
}

fn reusable_wf(id: &str, role_id: &str) -> pari::entities::workflow::TrackedReusableWorkflow {
    let mut steps = IndexMap::new();
    steps.insert(
        "Step1".to_string(),
        Step::Task {
            entity_ref: EntityRef::with_parent(
                "Step1",
                WorkflowParent::Workflow(EntityRef::new(id)),
            ),
            depends_on: None,
        },
    );
    ReusableWorkflow {
        entity_ref: EntityRef::new(id),
        name: id.to_string(),
        description: None,
        purpose: "Reusable".to_string(),
        raci: raci(role_id),
        states: workflow_states(),
        steps,
        intercepts: None,
        guidance: None,
        extensions: Extensions::default(),
    }
    .into()
}

/// Extracts the field-level error map, or panics on unexpected variants.
fn validation_errors(
    result: Result<(), ActivityError>,
) -> std::collections::HashMap<String, Vec<PrimitiveError>> {
    match result {
        Ok(()) => std::collections::HashMap::new(),
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
// Workflow cross-entity — raci roles
// ---------------------------------------------------------------------------

#[tokio::test]
async fn workflow_raci_role_exists_passes_cross_entity() {
    EntityServer::with(InMemorySubstrate::new(), || async {
        // Insert the role the raci references
        EntityClient::insert(TrackedEntity::from_role(role("pm")))
            .await
            .unwrap();

        let wf = simple_workflow("DeployPipeline", "pm", "Build");
        let result =
            run_validations::<Workflow>(&wf, &["raci"], &[ValidationKind::CrossEntity]).await;
        assert!(
            result.is_ok(),
            "Cross-entity validation should pass when raci role exists: {result:?}"
        );
    })
    .await;
}

#[tokio::test]
async fn workflow_raci_role_missing_fails_cross_entity() {
    EntityServer::with(InMemorySubstrate::new(), || async {
        // Do NOT insert role "ghost-role" — it doesn't exist

        let wf = simple_workflow("DeployPipeline", "ghost-role", "Build");
        let result =
            run_validations::<Workflow>(&wf, &["raci"], &[ValidationKind::CrossEntity]).await;
        let errors = validation_errors(result);
        assert!(
            !errors.is_empty(),
            "Expected cross-entity error when raci role is missing"
        );
        assert!(
            errors.keys().any(|k| k.contains("raci")),
            "Expected error keyed on 'raci', got: {errors:?}"
        );
    })
    .await;
}

// ---------------------------------------------------------------------------
// Workflow cross-entity — step entity refs
// ---------------------------------------------------------------------------

#[tokio::test]
async fn workflow_step_ref_exists_passes_cross_entity() {
    EntityServer::with(InMemorySubstrate::new(), || async {
        let role_id = "pm";
        let step_id = "Build";
        let wf_id = "DeployPipeline";
        let artifact_kind_id = "doc";

        // ArtifactKind must exist before inserting the task (task.artifact.kind refs it).
        EntityClient::insert(TrackedEntity::from_artifact_kind(artifact_kind(
            artifact_kind_id,
        )))
        .await
        .unwrap();
        EntityClient::insert(TrackedEntity::from_role(role(role_id)))
            .await
            .unwrap();
        EntityClient::insert(TrackedEntity::from_task(task(
            step_id,
            wf_id,
            artifact_kind_id,
        )))
        .await
        .unwrap();

        let wf = simple_workflow(wf_id, role_id, step_id);
        let result =
            run_validations::<Workflow>(&wf, &["steps"], &[ValidationKind::CrossEntity]).await;
        assert!(
            result.is_ok(),
            "Cross-entity validation should pass when step ref exists: {result:?}"
        );
    })
    .await;
}

#[tokio::test]
async fn workflow_step_ref_missing_fails_cross_entity() {
    EntityServer::with(InMemorySubstrate::new(), || async {
        let role_id = "pm";
        let step_id = "Build";
        let wf_id = "DeployPipeline";

        EntityClient::insert(TrackedEntity::from_role(role(role_id)))
            .await
            .unwrap();
        // Do NOT insert the task for "Build"

        let wf = simple_workflow(wf_id, role_id, step_id);
        let result =
            run_validations::<Workflow>(&wf, &["steps"], &[ValidationKind::CrossEntity]).await;
        let errors = validation_errors(result);
        assert!(
            !errors.is_empty(),
            "Expected cross-entity error when step task ref is missing"
        );
        assert!(
            errors.keys().any(|k| k.contains("steps")),
            "Expected error keyed on 'steps', got: {errors:?}"
        );
    })
    .await;
}

// ---------------------------------------------------------------------------
// Relay cross-entity — delegates_to
// ---------------------------------------------------------------------------

#[tokio::test]
async fn relay_delegates_to_exists_passes_cross_entity() {
    EntityServer::with(InMemorySubstrate::new(), || async {
        let role_id = "pm";
        let reusable_id = "ApprovalFlow";

        // The reusable workflow contains a step "Step1" referencing a task entity.
        // Insert the dependency chain in order: ArtifactKind → Role → Task → ReusableWorkflow.
        EntityClient::insert(TrackedEntity::from_artifact_kind(artifact_kind("doc")))
            .await
            .unwrap();
        EntityClient::insert(TrackedEntity::from_role(role(role_id)))
            .await
            .unwrap();
        EntityClient::insert(TrackedEntity::from_task(task("Step1", reusable_id, "doc")))
            .await
            .unwrap();
        EntityClient::insert(TrackedEntity::from_reusable_workflow(reusable_wf(
            reusable_id,
            role_id,
        )))
        .await
        .unwrap();

        let rel = relay("DelegateStep", "DeployPipeline", reusable_id);
        let result =
            run_validations::<Relay>(&rel, &["delegates_to"], &[ValidationKind::CrossEntity]).await;
        assert!(
            result.is_ok(),
            "Cross-entity validation should pass when delegates_to exists: {result:?}"
        );
    })
    .await;
}

#[tokio::test]
async fn relay_delegates_to_missing_fails_cross_entity() {
    EntityServer::with(InMemorySubstrate::new(), || async {
        // Do NOT insert the reusable workflow "ApprovalFlow"

        let rel = relay("DelegateStep", "DeployPipeline", "ApprovalFlow");
        let result =
            run_validations::<Relay>(&rel, &["delegates_to"], &[ValidationKind::CrossEntity]).await;
        let errors = validation_errors(result);
        assert!(
            !errors.is_empty(),
            "Expected cross-entity error when delegates_to is missing"
        );
        assert!(
            errors.keys().any(|k| k.contains("delegates_to")),
            "Expected error keyed on 'delegates_to', got: {errors:?}"
        );
    })
    .await;
}

// ---------------------------------------------------------------------------
// EntityClient::has_ref direct contract tests
// ---------------------------------------------------------------------------

#[tokio::test]
async fn has_ref_returns_true_for_existing_entity() {
    EntityServer::with(InMemorySubstrate::new(), || async {
        EntityClient::insert(TrackedEntity::from_role(role("eng-lead")))
            .await
            .unwrap();
        let any_ref = AnyEntityRef::Role(EntityRef::new("eng-lead"));
        let exists = EntityClient::has_ref(any_ref).await.unwrap();
        assert!(exists, "has_ref should return true for an inserted entity");
    })
    .await;
}

#[tokio::test]
async fn has_ref_returns_false_for_absent_entity() {
    EntityServer::with(InMemorySubstrate::new(), || async {
        let any_ref = AnyEntityRef::Role(EntityRef::new("ghost"));
        let exists = EntityClient::has_ref(any_ref).await.unwrap();
        assert!(
            !exists,
            "has_ref should return false for a non-existent entity"
        );
    })
    .await;
}
