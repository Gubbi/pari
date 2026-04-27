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

/// Workflow with a single `Step::Task` referencing an embedded task.
///
/// The task itself must be inserted separately — typically using
/// [`crate::fixtures::task::a_minimal_task`] with the same `id`
/// (workflow id) and a matching `task_id`.
pub fn a_workflow_with_task_step(
    id: &str,
    accountable_role_id: &str,
    task_id: &str,
) -> TrackedEntity {
    let raci = canonical_raci(accountable_role_id);
    let parent = WorkflowParent::Workflow(EntityRef::<Workflow>::new(id));
    let mut steps: IndexMap<String, Step> = IndexMap::new();
    steps.insert(
        "Design".to_string(),
        Step::Task {
            entity_ref: EntityRef::<Task, _>::with_parent(task_id, parent),
            depends_on: None,
        },
    );
    workflow(id, raci, two_state_with_done(), steps)
}

/// Workflow with a `Step::Task` followed by a `Step::Review`.
///
/// Adds the `Reviewing` state required when a Review step is present,
/// and points `on_reject` back at the task step so it satisfies
/// `on_reject_valid`.
pub fn a_workflow_with_task_and_review(
    id: &str,
    accountable_role_id: &str,
    task_id: &str,
    approver_role_id: &str,
) -> TrackedEntity {
    let raci = canonical_raci(accountable_role_id);
    let parent = WorkflowParent::Workflow(EntityRef::<Workflow>::new(id));
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
    workflow(id, raci, three_state_with_reviewing_and_done(), steps)
}

fn canonical_raci(role_id: &str) -> Raci {
    Raci {
        responsible: vec![EntityRef::<Role>::new(role_id)],
        accountable: EntityRef::<Role>::new(role_id),
        consulted: None,
        informed: None,
    }
}

fn two_state_with_done() -> Vec<WorkflowStateEntry> {
    vec![
        WorkflowStateEntry {
            id: "InProgress".to_string(),
            description: "Work in progress.".to_string(),
            semantic: None,
        },
        WorkflowStateEntry {
            id: "Done".to_string(),
            description: "Workflow complete.".to_string(),
            semantic: Some(WorkflowSemantic::Done),
        },
    ]
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
