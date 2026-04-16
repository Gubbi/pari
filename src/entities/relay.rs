use std::collections::HashMap;

use crate::{
    entities::workflow::ReusableWorkflow,
    entity::{EntityKind, EntityRef, WorkflowParent},
    types::{Extensions, HookCall, Raci, TaskTrigger},
};

#[derive(pari_macros::Entity)]
#[entity(kind = EntityKind::Relay, parent = WorkflowParent, schema = crate::validation::relay::relay_validation_schema)]
pub struct Relay {
    pub entity_ref: EntityRef<Relay, WorkflowParent>,
    pub name: String,
    pub description: Option<String>,
    pub purpose: String,
    pub raci: Option<Raci>,
    pub delegates_to: EntityRef<ReusableWorkflow>,
    pub briefing: Option<String>,
    pub debriefing: Option<String>,
    pub state_map: HashMap<String, StateMapEntry>,
    pub intercepts: Option<HashMap<TaskTrigger, HookCall>>,
    pub guidance: Option<String>,
    pub extensions: Extensions,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct StateMapEntry {
    pub maps_to: String,
    pub description: Option<String>,
    pub semantic: Option<StateMapSemantic>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum StateMapSemantic {
    Done,
    Blocked,
    Failed,
}
