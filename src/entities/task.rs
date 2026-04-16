use std::collections::HashMap;

use crate::{
    entity::{EntityKind, EntityRef, WorkflowParent},
    types::{Artifact, Extensions, HookCall, Raci, TaskStateEntry, TaskTrigger},
};

#[derive(pari_macros::Entity)]
#[entity(kind = EntityKind::Task, parent = WorkflowParent, schema = crate::validation::task::task_validation_schema)]
pub struct Task {
    pub entity_ref: EntityRef<Task, WorkflowParent>,
    pub name: String,
    pub description: Option<String>,
    pub purpose: String,
    pub instructions: Vec<String>,
    pub criteria: Vec<String>,
    pub raci: Option<Raci>,
    pub artifact: Artifact,
    pub states: Vec<TaskStateEntry>,
    pub intercepts: Option<HashMap<TaskTrigger, HookCall>>,
    pub guidance: Option<String>,
    pub extensions: Extensions,
}
