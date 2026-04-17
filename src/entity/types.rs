use std::collections::HashMap;

use crate::entity::{
    entities::{artifact_kind::ArtifactKind, hook::Hook, role::Role},
    EntityRef,
};

/// Open-ended metadata. Only `x-` prefixed keys are permitted (enforced by validation).
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct Extensions(pub HashMap<String, serde_json::Value>);

impl Extensions {
    pub fn new() -> Self {
        Self(HashMap::new())
    }
}

impl std::ops::Deref for Extensions {
    type Target = HashMap<String, serde_json::Value>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for Extensions {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl From<HashMap<String, serde_json::Value>> for Extensions {
    fn from(value: HashMap<String, serde_json::Value>) -> Self {
        Self(value)
    }
}

impl<'a> IntoIterator for &'a Extensions {
    type Item = (&'a String, &'a serde_json::Value);
    type IntoIter = std::collections::hash_map::Iter<'a, String, serde_json::Value>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

impl schemars::JsonSchema for Extensions {
    fn schema_name() -> String {
        "Extensions".to_string()
    }

    fn json_schema(_gen: &mut schemars::gen::SchemaGenerator) -> schemars::schema::Schema {
        use schemars::schema::{Schema, SchemaObject};

        let mut obj = SchemaObject::default();
        obj.object()
            .pattern_properties
            .insert("^x-".to_string(), Schema::Bool(true));
        Schema::Object(obj)
    }
}

/// Accountability assignment for workflows and tasks.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(deny_unknown_fields)]
pub struct Raci {
    pub responsible: Vec<EntityRef<Role>>,
    pub accountable: EntityRef<Role>,
    pub consulted: Option<Vec<EntityRef<Role>>>,
    pub informed: Option<Vec<EntityRef<Role>>>,
}

/// Task deliverable specification.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(deny_unknown_fields)]
pub struct Artifact {
    pub kind: EntityRef<ArtifactKind>,
    pub template: Option<String>,
}

/// Usage-site reference to a hook with optional input bindings.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(deny_unknown_fields)]
pub struct HookCall {
    pub hook: EntityRef<Hook>,
    pub with: Option<HashMap<String, String>>,
}

// --- Lifecycle state types ---

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(deny_unknown_fields)]
pub struct WorkflowStateEntry {
    #[schemars(regex(pattern = r"^[A-Z][A-Za-z0-9]*$"))]
    pub id: String,
    pub description: String,
    pub semantic: Option<WorkflowSemantic>,
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowSemantic {
    Reviewing,
    Done,
    Blocked,
    Failed,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(deny_unknown_fields)]
pub struct TaskStateEntry {
    #[schemars(regex(pattern = r"^[A-Z][A-Za-z0-9]*$"))]
    pub id: String,
    pub description: String,
    pub semantic: Option<TaskSemantic>,
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum TaskSemantic {
    Done,
    Blocked,
    Failed,
}

// --- Trigger enums ---

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    serde::Serialize,
    serde::Deserialize,
    schemars::JsonSchema,
)]
pub enum WorkflowTrigger {
    OnStart,
    OnDone,
    OnBlocked,
    OnFailed,
    OnReviewing,
    OnReject,
}

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    serde::Serialize,
    serde::Deserialize,
    schemars::JsonSchema,
)]
pub enum TaskTrigger {
    OnStart,
    OnDone,
    OnBlocked,
    OnFailed,
}
