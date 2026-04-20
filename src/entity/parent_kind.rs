use super::{
    entities::workflow::{EmbeddedWorkflow, ReusableWorkflow, Workflow},
    AnyEntityRef, EntityRef,
};
use crate::error::primitive::PrimitiveError;

pub(crate) mod private {
    pub trait Sealed {}
}

/// Marker trait for parent type parameters on EntityRef.
pub trait ParentKind:
    private::Sealed + Clone + PartialEq + Eq + std::hash::Hash + std::fmt::Debug
{
    fn serialize_parent<M: serde::ser::SerializeMap>(&self, map: &mut M) -> Result<(), M::Error>;
    fn deserialize_parent(
        parent: Option<serde_json::Value>,
        child_kind: &'static str,
    ) -> Result<Self, PrimitiveError>
    where
        Self: Sized;
    fn value(&self) -> Option<&Self>;
}

/// Top-level entities have no parent.
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
pub struct NoParent;

impl private::Sealed for NoParent {}

impl ParentKind for NoParent {
    fn serialize_parent<M: serde::ser::SerializeMap>(&self, _map: &mut M) -> Result<(), M::Error> {
        Ok(())
    }

    fn deserialize_parent(
        parent: Option<serde_json::Value>,
        child_kind: &'static str,
    ) -> Result<Self, PrimitiveError> {
        if parent.is_some() {
            return Err(PrimitiveError::unexpected_parent_on_top_level_entity(
                "unexpected parent on top-level entity",
                child_kind,
            ));
        }
        Ok(NoParent)
    }

    fn value(&self) -> Option<&Self> {
        None
    }
}

/// Closed parent hierarchy for embedded entities in the workflow tree.
#[derive(
    Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
#[serde(untagged)]
pub enum WorkflowParent {
    Workflow(EntityRef<Workflow, NoParent>),
    ReusableWorkflow(EntityRef<ReusableWorkflow, NoParent>),
    EmbeddedWorkflow(Box<EntityRef<EmbeddedWorkflow, WorkflowParent>>),
}

impl private::Sealed for WorkflowParent {}

impl ParentKind for WorkflowParent {
    fn serialize_parent<M: serde::ser::SerializeMap>(&self, map: &mut M) -> Result<(), M::Error> {
        map.serialize_entry("parent", self)
    }

    fn deserialize_parent(
        parent: Option<serde_json::Value>,
        child_kind: &'static str,
    ) -> Result<Self, PrimitiveError> {
        let parent = parent.ok_or_else(|| {
            PrimitiveError::missing_required_parent_object(
                "missing required parent object",
                child_kind,
            )
        })?;
        serde_json::from_value(parent.clone()).map_err(|err| {
            PrimitiveError::malformed_nested_parent_reference(
                "malformed nested parent reference",
                parent.to_string(),
                err.to_string(),
            )
        })
    }

    fn value(&self) -> Option<&Self> {
        Some(self)
    }
}

impl WorkflowParent {
    pub fn to_any_ref(&self) -> AnyEntityRef {
        match self {
            WorkflowParent::Workflow(r) => AnyEntityRef::Workflow(r.clone()),
            WorkflowParent::ReusableWorkflow(r) => AnyEntityRef::ReusableWorkflow(r.clone()),
            WorkflowParent::EmbeddedWorkflow(r) => AnyEntityRef::EmbeddedWorkflow((**r).clone()),
        }
    }
}
