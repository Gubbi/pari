use crate::entity::{AnyEntityRef, TrackedEntity};

/// A single tracked-entity change emitted by the store for persistence.
pub enum EntityChange<'a> {
    Added(&'a TrackedEntity),
    Modified(&'a TrackedEntity, Vec<&'static str>),
    Removed(&'a AnyEntityRef),
}
