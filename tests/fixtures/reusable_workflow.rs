//! Canonical [`ReusableWorkflow`] sample data for tests.

use indexmap::IndexMap;
use pari::{
    entities::{
        role::Role,
        workflow::{ReusableWorkflow, Step, TrackedReusableWorkflow},
    },
    entity::{EntityRef, TrackedEntity},
    types::{Raci, WorkflowSemantic, WorkflowStateEntry},
};

/// Reusable workflow with a single `Step::Review`.
///
/// Both `accountable_role_id` (raci) and `approver_role_id` (review
/// approver) must already be inserted in the substrate. States are
/// `[InProgress, Reviewing, Done]` so a relay's `state_map` can name
/// any of those as `maps_to`.
pub fn a_reusable_workflow_with_review_step(
    id: &str,
    accountable_role_id: &str,
    approver_role_id: &str,
) -> TrackedEntity {
    let raci = Raci {
        responsible: vec![EntityRef::<Role>::new(accountable_role_id)],
        accountable: EntityRef::<Role>::new(accountable_role_id),
        consulted: None,
        informed: None,
    };
    let mut steps: IndexMap<String, Step> = IndexMap::new();
    steps.insert(
        "Review".to_string(),
        Step::Review {
            approver: vec![EntityRef::<Role>::new(approver_role_id)],
            on_reject: "Review".to_string(),
        },
    );
    TrackedEntity::from_reusable_workflow(TrackedReusableWorkflow::from(ReusableWorkflow {
        entity_ref: EntityRef::new(id),
        name: "Approval Loop".to_string(),
        description: Some("A reusable approval flow.".to_string()),
        purpose: "Standardize approval handoffs.".to_string(),
        raci,
        states: vec![
            WorkflowStateEntry {
                id: "InProgress".to_string(),
                description: "Awaiting review.".to_string(),
                semantic: None,
            },
            WorkflowStateEntry {
                id: "Reviewing".to_string(),
                description: "Under review.".to_string(),
                semantic: Some(WorkflowSemantic::Reviewing),
            },
            WorkflowStateEntry {
                id: "Done".to_string(),
                description: "Approval complete.".to_string(),
                semantic: Some(WorkflowSemantic::Done),
            },
        ],
        steps,
        intercepts: None,
        guidance: None,
        extensions: Default::default(),
    }))
}
