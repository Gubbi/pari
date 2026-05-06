//! Canonical [`Relay`] sample data for tests.

use std::collections::HashMap;

use indexmap::IndexMap;
use pari::{
    entities::{
        relay::{Relay, StateMapEntry},
        role::Role,
        workflow::{ReusableWorkflow, Step, Workflow},
    },
    entity::{EntityRef, WorkflowParent},
    types::Raci,
};

/// Bare relay embedded under `workflow_id`, delegating to
/// `reusable_workflow_id`. The relay's `state_map` names `InProgress`
/// and `Done` as `maps_to` targets — both must be states on the chosen
/// reusable workflow (true of [`crate::fixtures::reusable_workflow::a_reusable_workflow_with_review_step`]).
pub fn a_minimal_relay(
    id: &str,
    workflow_id: &str,
    accountable_role_id: &str,
    reusable_workflow_id: &str,
) -> Relay {
    let parent = WorkflowParent::Workflow(EntityRef::<Workflow>::new(workflow_id));
    let mut state_map: HashMap<String, StateMapEntry> = HashMap::new();
    state_map.insert(
        "Pending".to_string(),
        StateMapEntry {
            maps_to: "InProgress".to_string(),
            description: None,
            semantic: None,
        },
    );
    state_map.insert(
        "Complete".to_string(),
        StateMapEntry {
            maps_to: "Done".to_string(),
            description: None,
            semantic: None,
        },
    );
    let raci = Raci {
        responsible: vec![EntityRef::<Role>::new(accountable_role_id)],
        accountable: EntityRef::<Role>::new(accountable_role_id),
        consulted: None,
        informed: None,
    };
    Relay {
        entity_ref: EntityRef::with_parent(id, parent),
        name: "Approval Handoff".to_string(),
        description: None,
        purpose: "Hand off approval to a shared sub-procedure.".to_string(),
        raci: Some(raci),
        delegates_to: EntityRef::<ReusableWorkflow>::new(reusable_workflow_id),
        briefing: None,
        debriefing: None,
        state_map,
        intercepts: None,
        guidance: None,
        extensions: Default::default(),
    }
}

/// Steps payload for a parent workflow's final shape: a single
/// `Step::Relay` referencing a relay whose parent is that workflow.
pub fn relay_step(relay_id: &str, parent_workflow_id: &str) -> IndexMap<String, Step> {
    let parent = WorkflowParent::Workflow(EntityRef::<Workflow>::new(parent_workflow_id));
    let relay_ref = EntityRef::<Relay, _>::with_parent(relay_id, parent);
    let mut steps: IndexMap<String, Step> = IndexMap::new();
    steps.insert(
        "Handoff".to_string(),
        Step::Relay {
            entity_ref: relay_ref,
            depends_on: None,
        },
    );
    steps
}
