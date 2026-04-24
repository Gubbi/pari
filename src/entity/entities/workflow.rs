use std::collections::HashMap;

use indexmap::IndexMap;

use crate::entity::{
    entities::{relay::Relay, role::Role, task::Task},
    types::{Extensions, HookCall, Raci, WorkflowStateEntry, WorkflowTrigger},
    EntityKind, EntityRef, WorkflowParent,
};

/// One position in a workflow's ordered step map.
///
/// `Step` is structural glue, not an entity: it has no `EntityRef` and no
/// tracked companion. Each variant carries a reference to the embedded entity
/// that actually runs at that position ([`Task`], [`Relay`],
/// [`EmbeddedWorkflow`]), plus per-step scheduling metadata like
/// `depends_on`. The `Review` variant is the exception — it is resolved
/// inline against roles, not against a separate entity.
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

/// A top-level procedure that a team checks out and executes.
///
/// `Workflow` is the unit of delivery: it owns a set of lifecycle states, an
/// ordered step map, RACI assignments, and optional intercepts. It is
/// top-level (`NoParent`) because it is the root of an execution — every
/// other workflow-family entity is either embedded underneath one, or
/// invoked from one via a [`crate::entity::entities::relay::Relay`].
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

/// A workflow meant to be invoked by a [`Relay`], not scheduled directly.
///
/// `ReusableWorkflow` has the same shape as [`Workflow`] but a different role
/// in the system: it is a library definition. Teams publish one so other
/// workflows can delegate to it, which is how recurring procedures (review,
/// sign-off, standard sub-processes) are kept DRY.
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

/// A workflow nested inline inside another workflow's step map.
///
/// Unlike a [`crate::entity::entities::relay::Relay`] — which points
/// at a separately-defined [`ReusableWorkflow`] — an `EmbeddedWorkflow` is
/// authored in place and identified relative to its parent. This makes it
/// the right choice for one-off nested procedures that do not need to be
/// shared. Its [`WorkflowParent`] is itself recursive so embeddings can nest
/// arbitrarily deep.
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
