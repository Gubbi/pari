//! The store → substrate persist handoff type.

use crate::entity::{AnyEntityRef, TrackedEntity};

/// A single tracked-entity change emitted by the store for persistence.
///
/// `Modified` carries the dirty-field list so substrates that support
/// partial writes can persist only what changed. Substrates that only
/// support full-entity writes can ignore the field list.
pub enum EntityChange {
    /// Entity inserted since the last persist; substrate should create.
    Added(TrackedEntity),
    /// Existing entity with committed edits on the listed dirty fields.
    Modified(TrackedEntity, Vec<&'static str>),
    /// Entity removed since the last persist; substrate should delete.
    Removed(AnyEntityRef),
}
