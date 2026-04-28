//! Canonical [`Task`] sample data for tests.

use pari::{
    entities::{
        artifact_kind::ArtifactKind,
        task::{Task, TrackedTask},
        workflow::Workflow,
    },
    entity::{EntityRef, TrackedEntity, WorkflowParent},
    types::{Artifact, TaskSemantic, TaskStateEntry},
};

/// Bare task embedded under `workflow_id`, producing an artifact of
/// kind `artifact_kind_id`. Required fields populated; raci, intercepts,
/// and guidance left absent so the workflow's raci flows through.
pub fn a_minimal_task(id: &str, workflow_id: &str, artifact_kind_id: &str) -> TrackedEntity {
    let parent = WorkflowParent::Workflow(EntityRef::<Workflow>::new(workflow_id));
    a_minimal_task_with_parent(id, parent, artifact_kind_id)
}

/// Like [`a_minimal_task`] but lets the caller construct the parent
/// chain directly — used when the task lives under an embedded
/// workflow rather than a top-level one.
pub fn a_minimal_task_with_parent(
    id: &str,
    parent: WorkflowParent,
    artifact_kind_id: &str,
) -> TrackedEntity {
    TrackedEntity::from_task(TrackedTask::from(Task {
        entity_ref: EntityRef::with_parent(id, parent),
        name: "Design Doc Draft".to_string(),
        description: None,
        purpose: "test purpose".to_string(),
        instructions: vec!["Outline the proposal.".to_string()],
        criteria: vec!["Reviewed by accountable role.".to_string()],
        raci: None,
        artifact: Artifact {
            kind: EntityRef::<ArtifactKind>::new(artifact_kind_id),
            template: Some("# Design Doc\n\n_Outline goes here._\n".to_string()),
        },
        states: vec![
            TaskStateEntry {
                id: "InProgress".to_string(),
                description: "Work in progress.".to_string(),
                semantic: None,
            },
            TaskStateEntry {
                id: "Done".to_string(),
                description: "Task complete.".to_string(),
                semantic: Some(TaskSemantic::Done),
            },
        ],
        intercepts: None,
        guidance: None,
        extensions: Default::default(),
    }))
}
