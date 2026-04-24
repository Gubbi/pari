use std::collections::HashMap;

use crate::entity::{
    types::{Artifact, Extensions, HookCall, Raci, TaskStateEntry, TaskTrigger},
    EntityKind, EntityRef, WorkflowParent,
};

/// A leaf unit of work inside a workflow that produces a single artifact.
///
/// `Task` is the atomic thing a role actually executes: it carries
/// instructions, acceptance criteria, a declared deliverable, and its own
/// lifecycle states. Tasks are embedded — their identity includes the
/// enclosing workflow via [`WorkflowParent`] — because the same task id
/// under two workflows is two distinct tasks with independent state.
#[derive(
    Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, pari_macros::Entity,
)]
#[schemars(deny_unknown_fields)]
#[entity(kind = EntityKind::Task, parent = WorkflowParent, schema = crate::validation::task::task_validation_schema)]
pub struct Task {
    pub entity_ref: EntityRef<Task, WorkflowParent>,
    pub name: String,
    pub description: Option<String>,
    pub purpose: String,
    #[schemars(length(min = 1))]
    pub instructions: Vec<String>,
    #[schemars(length(min = 1))]
    pub criteria: Vec<String>,
    pub raci: Option<Raci>,
    pub artifact: Artifact,
    #[schemars(length(min = 2))]
    pub states: Vec<TaskStateEntry>,
    pub intercepts: Option<HashMap<TaskTrigger, HookCall>>,
    pub guidance: Option<String>,
    #[serde(flatten)]
    pub extensions: Extensions,
}
