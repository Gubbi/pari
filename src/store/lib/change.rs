use crate::entity::{AnyEntityRef, TrackedEntity};

/// A single tracked-entity change emitted by the store for persistence.
pub enum EntityChange {
    Added(TrackedEntity),
    Modified(TrackedEntity, Vec<&'static str>),
    Removed(AnyEntityRef),
}
