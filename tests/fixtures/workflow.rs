//! Canonical [`Workflow`] sample data for tests.

use std::collections::HashMap;

use indexmap::IndexMap;
use pari::{
    entities::{
        role::Role,
        task::Task,
        workflow::{Step, Workflow},
    },
    entity::{EntityRef, WorkflowParent},
    types::{HookCall, Raci, WorkflowSemantic, WorkflowStateEntry, WorkflowTrigger},
};

/// Workflow shell with no steps yet, used as the first insertion when
/// authoring a workflow iteratively.
///
/// Embedded entities (tasks, relays, embedded workflows) are authored
/// next with this workflow as their parent. Once those exist, callers
/// `set_steps` to the final shape via [`task_and_review_steps`] or a
/// custom payload.
///
/// The state list is `[InProgress, InReview, Done]` so the canonical
/// final shape with a `Step::Review` does not also need a `set_states`
/// in the same modify cycle.
pub fn a_workflow_with_empty_steps(id: &str, accountable_role_id: &str) -> Workflow {
    let raci = canonical_raci(accountable_role_id);
    workflow(
        id,
        raci,
        three_state_with_reviewing_and_done(),
        IndexMap::new(),
        None,
    )
}

/// Workflow shell with no steps and the given lifecycle intercepts.
///
/// Each hook referenced via [`HookCall::hook`] must already exist when
/// the workflow is committed.
pub fn a_workflow_with_intercepts(
    id: &str,
    accountable_role_id: &str,
    intercepts: HashMap<WorkflowTrigger, HookCall>,
) -> Workflow {
    let raci = canonical_raci(accountable_role_id);
    workflow(
        id,
        raci,
        three_state_with_reviewing_and_done(),
        IndexMap::new(),
        Some(intercepts),
    )
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
    intercepts: Option<HashMap<WorkflowTrigger, HookCall>>,
) -> Workflow {
    Workflow {
        entity_ref: EntityRef::new(id),
        name: "Design Workflow".to_string(),
        description: Some("A workflow for tests.".to_string()),
        purpose: "Drive a single design through review.".to_string(),
        raci,
        states,
        steps,
        intercepts,
        guidance: None,
        extensions: Default::default(),
    }
}
