use std::collections::HashMap;

use indexmap::IndexMap;

use crate::entity::{
    entities::{relay::Relay, role::Role, task::Task},
    types::{Extensions, HookCall, Raci, WorkflowStateEntry, WorkflowTrigger},
    EntityKind, EntityRef, WorkflowParent,
};

/// A step inside a workflow. Not an entity — no EntityRef, no derive(Entity).
#[derive(
    Debug,
    Clone,
    serde::Serialize,
    serde::Deserialize,
    schemars::JsonSchema,
    pari_macros::CollectRefs,
)]
#[schemars(deny_unknown_fields)]
pub enum Step {
    Task {
        entity_ref: EntityRef<Task, WorkflowParent>,
        depends_on: Option<Vec<String>>,
    },
    Relay {
        entity_ref: EntityRef<Relay, WorkflowParent>,
        depends_on: Option<Vec<String>>,
    },
    EmbeddedWorkflow {
        entity_ref: EntityRef<EmbeddedWorkflow, WorkflowParent>,
        depends_on: Option<Vec<String>>,
    },
    Review {
        approver: Vec<EntityRef<Role>>,
        on_reject: String,
    },
}

#[derive(
    Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, pari_macros::Entity,
)]
#[schemars(deny_unknown_fields)]
#[entity(kind = EntityKind::Workflow, schema = crate::validation::workflow::workflow_validation_schema)]
pub struct Workflow {
    pub entity_ref: EntityRef<Workflow>,
    pub name: String,
    pub description: Option<String>,
    pub purpose: String,
    pub raci: Raci,
    #[schemars(length(min = 2))]
    pub states: Vec<WorkflowStateEntry>,
    #[schemars(length(min = 1))]
    pub steps: IndexMap<String, Step>,
    pub intercepts: Option<HashMap<WorkflowTrigger, HookCall>>,
    pub guidance: Option<String>,
    #[serde(flatten)]
    pub extensions: Extensions,
}

#[derive(
    Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, pari_macros::Entity,
)]
#[schemars(deny_unknown_fields)]
#[entity(kind = EntityKind::ReusableWorkflow, schema = crate::validation::workflow::reusable_workflow_validation_schema)]
pub struct ReusableWorkflow {
    pub entity_ref: EntityRef<ReusableWorkflow>,
    pub name: String,
    pub description: Option<String>,
    pub purpose: String,
    pub raci: Raci,
    #[schemars(length(min = 2))]
    pub states: Vec<WorkflowStateEntry>,
    #[schemars(length(min = 1))]
    pub steps: IndexMap<String, Step>,
    pub intercepts: Option<HashMap<WorkflowTrigger, HookCall>>,
    pub guidance: Option<String>,
    #[serde(flatten)]
    pub extensions: Extensions,
}

#[derive(
    Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, pari_macros::Entity,
)]
#[schemars(deny_unknown_fields)]
#[entity(kind = EntityKind::EmbeddedWorkflow, parent = WorkflowParent, schema = crate::validation::workflow::embedded_workflow_validation_schema)]
pub struct EmbeddedWorkflow {
    pub entity_ref: EntityRef<EmbeddedWorkflow, WorkflowParent>,
    pub name: String,
    pub description: Option<String>,
    pub purpose: String,
    pub raci: Option<Raci>,
    #[schemars(length(min = 2))]
    pub states: Vec<WorkflowStateEntry>,
    #[schemars(length(min = 1))]
    pub steps: IndexMap<String, Step>,
    pub intercepts: Option<HashMap<WorkflowTrigger, HookCall>>,
    pub guidance: Option<String>,
    #[serde(flatten)]
    pub extensions: Extensions,
}
