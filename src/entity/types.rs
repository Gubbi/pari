//! Shared value types embedded inside entities.
//!
//! These are plain serde structs and enums — not entities. They have no
//! identity, no tracked companion, and no lifecycle; they exist only as
//! fields on entities (`Raci` on a task, `Extensions` on any entity, …).
//! Every type here implements [`CollectRefs`](super::collect_refs::CollectRefs)
//! so entity refs buried inside them surface uniformly with a dot-notation
//! path.

use std::collections::HashMap;

use crate::entity::{
    entities::{artifact_kind::ArtifactKind, hook::Hook, role::Role},
    EntityRef,
};

/// Open-ended metadata.
///
/// On the wire, extension keys carry an `x-` prefix; in memory, the
/// prefix is stripped so the bag holds bare keys. Serialization adds
/// the prefix back. Non-`x-` prefixed keys appearing in input JSON are
/// not absorbed here — the schema gate at the input boundary rejects
/// them before this type ever sees them.
#[derive(Debug, Clone, Default, pari_macros::CollectRefs)]
pub struct Extensions(pub HashMap<String, serde_json::Value>);

impl Extensions {
    pub fn new() -> Self {
        Self(HashMap::new())
    }
}

const X_PREFIX: &str = "x-";

impl serde::Serialize for Extensions {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(Some(self.0.len()))?;
        for (k, v) in &self.0 {
            map.serialize_entry(&format!("{X_PREFIX}{k}"), v)?;
        }
        map.end()
    }
}

impl<'de> serde::Deserialize<'de> for Extensions {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        use std::fmt;

        struct Visitor;

        impl<'de> serde::de::Visitor<'de> for Visitor {
            type Value = Extensions;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                write!(f, "a map of x- prefixed extension entries")
            }

            fn visit_map<A: serde::de::MapAccess<'de>>(
                self,
                mut map: A,
            ) -> Result<Extensions, A::Error> {
                let mut bag: HashMap<String, serde_json::Value> = HashMap::new();
                while let Some(key) = map.next_key::<String>()? {
                    let value: serde_json::Value = map.next_value()?;
                    if let Some(stripped) = key.strip_prefix(X_PREFIX) {
                        bag.insert(stripped.to_string(), value);
                    }
                    // Non-`x-` keys are dropped here; the schema gate
                    // at the input boundary is the authoritative
                    // rejector. Until that gate runs, dropping is the
                    // safest in-memory outcome.
                }
                Ok(Extensions(bag))
            }
        }

        deserializer.deserialize_map(Visitor)
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
        // Reject non-`x-` keys at the schema boundary.
        obj.object().additional_properties = Some(Box::new(Schema::Bool(false)));
        Schema::Object(obj)
    }
}

/// Accountability assignment for workflows and tasks.
#[derive(
    Debug,
    Clone,
    serde::Serialize,
    serde::Deserialize,
    schemars::JsonSchema,
    pari_macros::CollectRefs,
)]
#[schemars(deny_unknown_fields)]
pub struct Raci {
    pub responsible: Vec<EntityRef<Role>>,
    pub accountable: EntityRef<Role>,
    pub consulted: Option<Vec<EntityRef<Role>>>,
    pub informed: Option<Vec<EntityRef<Role>>>,
}

/// Task deliverable specification.
#[derive(
    Debug,
    Clone,
    serde::Serialize,
    serde::Deserialize,
    schemars::JsonSchema,
    pari_macros::CollectRefs,
)]
#[schemars(deny_unknown_fields)]
pub struct Artifact {
    pub kind: EntityRef<ArtifactKind>,
    pub template: Option<String>,
}

/// Usage-site reference to a hook with optional input bindings.
#[derive(
    Debug,
    Clone,
    serde::Serialize,
    serde::Deserialize,
    schemars::JsonSchema,
    pari_macros::CollectRefs,
)]
#[schemars(deny_unknown_fields)]
pub struct HookCall {
    pub hook: EntityRef<Hook>,
    pub with: Option<HashMap<String, String>>,
}

// --- Lifecycle state types ---

#[derive(
    Debug,
    Clone,
    serde::Serialize,
    serde::Deserialize,
    schemars::JsonSchema,
    pari_macros::CollectRefs,
)]
#[schemars(deny_unknown_fields)]
pub struct WorkflowStateEntry {
    #[schemars(regex(pattern = r"^[A-Z][A-Za-z0-9]*$"))]
    pub id: String,
    pub description: String,
    pub semantic: Option<WorkflowSemantic>,
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
pub enum WorkflowSemantic {
    Reviewing,
    Done,
    Blocked,
    Failed,
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
pub struct TaskStateEntry {
    #[schemars(regex(pattern = r"^[A-Z][A-Za-z0-9]*$"))]
    pub id: String,
    pub description: String,
    pub semantic: Option<TaskSemantic>,
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
    pari_macros::CollectRefs,
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
    pari_macros::CollectRefs,
)]
pub enum TaskTrigger {
    OnStart,
    OnDone,
    OnBlocked,
    OnFailed,
}
