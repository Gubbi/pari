//! Entity-layer composition for identity, references, and plain entity types.
//!
//! This module owns the domain shapes the rest of Pari operates on. It is the
//! sole formal layer with no orchestration tier: every boundary emits
//! [`PrimitiveError`](crate::error::primitive::PrimitiveError) directly.
//!
//! See the L3 design doc at `docs/design/layers/entities.md` for the shape of
//! the layer, the identity model, and the generation contract.

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
