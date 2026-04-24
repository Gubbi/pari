//! Caller-facing async API over the entity server.
//!
//! Every external consumer drives Pari through this layer. Three surfaces
//! cover the full interaction:
//!
//! - [`EntityClient`] — typed operations keyed by `AnyEntityRef`.
//! - [`TrackedEntity`](crate::entity::TrackedEntity) methods — `commit` and
//!   `undo_checkout` on a checked-out entity.
//! - `#[derive(Entity)]`-generated accessors and setters — transparent
//!   per-field load and setter-time validation.
//!
//! No entity state lives here: the layer is stateless plumbing that builds
//! store messages, awaits replies, and forwards typed results to callers.
//! Setters additionally run structural
//! and semantic [`validation`](crate::validation) synchronously against a
//! candidate before swapping the field.
//!
//! See `docs/design/layers/workspace.md` for the L3 design.

mod client;
mod lib;
mod tracked_entity;

pub use client::EntityClient;
