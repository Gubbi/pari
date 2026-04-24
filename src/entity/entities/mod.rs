//! Plain entity definitions — one module per entity type.
//!
//! Each entity is a plain serde struct with `#[derive(Entity)]`; the tracked
//! companion, accessors, and dispatch glue are generated. See
//! `docs/design/layers/entities.md` for the full catalog and parent
//! assignments.

pub mod artifact_kind;
pub mod hook;
pub mod relay;
pub mod role;
pub mod task;
pub mod team;
pub mod workflow;
