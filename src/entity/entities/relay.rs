use std::collections::HashMap;

use crate::entity::{
    entities::workflow::ReusableWorkflow,
    types::{Extensions, HookCall, Raci, TaskTrigger},
    EntityKind, EntityRef, WorkflowParent,
};

/// A handoff step that delegates a portion of a workflow to a
/// [`ReusableWorkflow`].
///
/// Relays let a workflow plug in a standard sub-procedure — code review,
/// procurement, onboarding — without inlining or copying it. `delegates_to`
/// names the reusable workflow to run; `state_map` translates the callee's
/// lifecycle states back into the caller's vocabulary so the relay looks like
/// any other step from the outside.
#[derive(
    Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, pari_macros::Entity,
)]
#[schemars(deny_unknown_fields)]
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
    #[schemars(length(min = 1))]
    pub state_map: HashMap<String, StateMapEntry>,
    pub intercepts: Option<HashMap<TaskTrigger, HookCall>>,
    pub guidance: Option<String>,
    #[serde(flatten)]
    pub extensions: Extensions,
}

#[derive(
    Debug,
    Clone,
    serde::Serialize,
    serde::Deserialize,
    schemars::JsonSchema,
    pari_macros::CollectRefs,
)]
#[schemars(deny_unknown_fields)]
pub struct StateMapEntry {
    pub maps_to: String,
    pub description: Option<String>,
    pub semantic: Option<StateMapSemantic>,
}

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    serde::Serialize,
    serde::Deserialize,
    schemars::JsonSchema,
    pari_macros::CollectRefs,
)]
#[serde(rename_all = "snake_case")]
pub enum StateMapSemantic {
    Done,
    Blocked,
    Failed,
}
