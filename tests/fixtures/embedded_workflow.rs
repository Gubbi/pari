//! Canonical [`EmbeddedWorkflow`] sample data and step helpers.

use indexmap::IndexMap;
use pari::{
    entities::{
        role::Role,
        task::Task,
        workflow::{EmbeddedWorkflow, Step, TrackedEmbeddedWorkflow, Workflow},
    },
    entity::{EntityRef, TrackedEntity, WorkflowParent},
    types::{Raci, WorkflowSemantic, WorkflowStateEntry},
};

/// Embedded workflow shell, parented under a top-level workflow, with
/// no steps. Insertable as soon as the parent workflow exists.
pub fn a_minimal_embedded_workflow(
    id: &str,
    parent_workflow_id: &str,
    accountable_role_id: &str,
) -> TrackedEntity {
    let parent = WorkflowParent::Workflow(EntityRef::<Workflow>::new(parent_workflow_id));
    let raci = Raci {
        responsible: vec![EntityRef::<Role>::new(accountable_role_id)],
        accountable: EntityRef::<Role>::new(accountable_role_id),
        consulted: None,
        informed: None,
    };
    TrackedEntity::from_embedded_workflow(TrackedEmbeddedWorkflow::from(EmbeddedWorkflow {
        entity_ref: EntityRef::with_parent(id, parent),
        name: "Onboarding".to_string(),
        description: Some("A nested onboarding flow.".to_string()),
        purpose: "Drive a single onboarding pass.".to_string(),
        raci: Some(raci),
        states: vec![
            WorkflowStateEntry {
                id: "InProgress".to_string(),
                description: "Work in progress.".to_string(),
                semantic: None,
            },
            WorkflowStateEntry {
                id: "Done".to_string(),
                description: "Embedded workflow complete.".to_string(),
                semantic: Some(WorkflowSemantic::Done),
            },
        ],
        steps: IndexMap::new(),
        intercepts: None,
        guidance: None,
        extensions: Default::default(),
    }))
}

/// Steps payload for the embedded workflow's final shape: a single
/// `Step::Task` referencing a task whose parent is this embedded
/// workflow.
pub fn task_step_for_embedded(
    task_id: &str,
    embedded_id: &str,
    parent_workflow_id: &str,
) -> IndexMap<String, Step> {
    let parent_workflow = WorkflowParent::Workflow(EntityRef::<Workflow>::new(parent_workflow_id));
    let embedded_ref = EntityRef::<EmbeddedWorkflow, _>::with_parent(embedded_id, parent_workflow);
    let task_parent = WorkflowParent::EmbeddedWorkflow(Box::new(embedded_ref));
    let task_ref = EntityRef::<Task, _>::with_parent(task_id, task_parent);
    let mut steps: IndexMap<String, Step> = IndexMap::new();
    steps.insert(
        "Welcome".to_string(),
        Step::Task {
            entity_ref: task_ref,
            depends_on: None,
        },
    );
    steps
}

/// Steps payload for the parent workflow's final shape: a single
/// `Step::EmbeddedWorkflow` referencing the embedded workflow.
pub fn embedded_workflow_step(
    embedded_id: &str,
    parent_workflow_id: &str,
) -> IndexMap<String, Step> {
    let parent = WorkflowParent::Workflow(EntityRef::<Workflow>::new(parent_workflow_id));
    let embedded_ref = EntityRef::<EmbeddedWorkflow, _>::with_parent(embedded_id, parent);
    let mut steps: IndexMap<String, Step> = IndexMap::new();
    steps.insert(
        "Onboarding".to_string(),
        Step::EmbeddedWorkflow {
            entity_ref: embedded_ref,
            depends_on: None,
        },
    );
    steps
}
