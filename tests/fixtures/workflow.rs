//! Canonical [`Workflow`] sample data for tests.

use indexmap::IndexMap;
use pari::{
    entities::{
        role::Role,
        task::Task,
        workflow::{Step, TrackedWorkflow, Workflow},
    },
    entity::{EntityRef, TrackedEntity, WorkflowParent},
    types::{Raci, WorkflowSemantic, WorkflowStateEntry},
};

/// Workflow shell with a single `Step::Review`, used as the first
/// insertion when authoring a workflow iteratively.
///
/// The Review placeholder satisfies the "≥1 step" rule on insert so
/// embedded entities (tasks, relays, embedded workflows) can be authored
/// next with this workflow as their parent. Once those exist, callers
/// `set_steps` to the final shape via [`task_and_review_steps`] or a
/// custom payload.
pub fn a_workflow_with_review_placeholder(
    id: &str,
    accountable_role_id: &str,
    approver_role_id: &str,
) -> TrackedEntity {
    let raci = canonical_raci(accountable_role_id);
    let mut steps: IndexMap<String, Step> = IndexMap::new();
    steps.insert(
        "Review".to_string(),
        Step::Review {
            approver: vec![EntityRef::<Role>::new(approver_role_id)],
            on_reject: "Review".to_string(),
        },
    );
    workflow(id, raci, three_state_with_reviewing_and_done(), steps)
}

/// Steps payload for a workflow whose final shape is one `Step::Task`
/// followed by one `Step::Review`. The task and the review approver
/// must already exist when the workflow is committed.
pub fn task_and_review_steps(
    task_id: &str,
    workflow_id: &str,
    approver_role_id: &str,
) -> IndexMap<String, Step> {
    let parent = WorkflowParent::Workflow(EntityRef::<Workflow>::new(workflow_id));
    let mut steps: IndexMap<String, Step> = IndexMap::new();
    steps.insert(
        "Design".to_string(),
        Step::Task {
            entity_ref: EntityRef::<Task, _>::with_parent(task_id, parent),
            depends_on: None,
        },
    );
    steps.insert(
        "Review".to_string(),
        Step::Review {
            approver: vec![EntityRef::<Role>::new(approver_role_id)],
            on_reject: "Design".to_string(),
        },
    );
    steps
}

fn canonical_raci(role_id: &str) -> Raci {
    Raci {
        responsible: vec![EntityRef::<Role>::new(role_id)],
        accountable: EntityRef::<Role>::new(role_id),
        consulted: None,
        informed: None,
    }
}

fn three_state_with_reviewing_and_done() -> Vec<WorkflowStateEntry> {
    vec![
        WorkflowStateEntry {
            id: "InProgress".to_string(),
            description: "Work in progress.".to_string(),
            semantic: None,
        },
        WorkflowStateEntry {
            id: "InReview".to_string(),
            description: "Awaiting approver review.".to_string(),
            semantic: Some(WorkflowSemantic::Reviewing),
        },
        WorkflowStateEntry {
            id: "Done".to_string(),
            description: "Workflow complete.".to_string(),
            semantic: Some(WorkflowSemantic::Done),
        },
    ]
}

fn workflow(
    id: &str,
    raci: Raci,
    states: Vec<WorkflowStateEntry>,
    steps: IndexMap<String, Step>,
) -> TrackedEntity {
    TrackedEntity::from_workflow(TrackedWorkflow::from(Workflow {
        entity_ref: EntityRef::new(id),
        name: "Design Workflow".to_string(),
        description: Some("A workflow for tests.".to_string()),
        purpose: "Drive a single design through review.".to_string(),
        raci,
        states,
        steps,
        intercepts: None,
        guidance: None,
        extensions: Default::default(),
    }))
}
