//! Entity-layer composition for identity, references, and plain entity types.

pub mod collect_refs;
pub mod entities;
mod entity_kind;
mod entity_ref;
mod entity_trait;
mod parent_kind;
pub mod tracked;
pub mod types;

pub use entity_kind::*;
pub use entity_ref::EntityRef;
pub use entity_trait::{Entity, Tracked, TrackedFor};
pub use parent_kind::{NoParent, ParentKind, WorkflowParent};

pub use crate::validation::ValidationSchema;

#[cfg(test)]
mod tests;
