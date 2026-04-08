use std::collections::HashMap;
use crate::entity::EntityRef;

/// Open-ended metadata. Only `x-` prefixed keys are permitted (enforced by validation).
pub type Extensions = HashMap<String, serde_json::Value>;

/// Accountability assignment for workflows and tasks.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Raci {
    pub responsible: Vec<EntityRef<crate::entities::role::Role>>,
    pub accountable: EntityRef<crate::entities::role::Role>,
    pub consulted: Option<Vec<EntityRef<crate::entities::role::Role>>>,
    pub informed: Option<Vec<EntityRef<crate::entities::role::Role>>>,
}

/// Task deliverable specification.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Artifact {
    pub kind: EntityRef<crate::entities::artifact_kind::ArtifactKind>,
    pub template: Option<String>,
}

/// Usage-site reference to a hook with optional input bindings.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct HookCall {
    pub hook: EntityRef<crate::entities::hook::Hook>,
    pub with: Option<HashMap<String, String>>,
}

// --- Lifecycle state types ---

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct WorkflowStateEntry {
    pub id: String,
    pub description: String,
    pub semantic: Option<WorkflowSemantic>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum WorkflowSemantic {
    Reviewing,
    Done,
    Blocked,
    Failed,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TaskStateEntry {
    pub id: String,
    pub description: String,
    pub semantic: Option<TaskSemantic>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum TaskSemantic {
    Done,
    Blocked,
    Failed,
}

// --- Trigger enums ---

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum WorkflowTrigger {
    OnStart,
    OnDone,
    OnBlocked,
    OnFailed,
    OnReviewing,
    OnReject,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum TaskTrigger {
    OnStart,
    OnDone,
    OnBlocked,
    OnFailed,
}
