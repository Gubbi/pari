//! Wire types for the workspace ↔ `StoreServer` dispatch surface.
//!
//! Every caller operation from [`workspace`](crate::workspace) maps to
//! exactly one [`WorkspaceRequest`] variant; the dispatcher returns a
//! [`WorkspaceResponse`] carrying either the typed payload or an
//! [`ActivityError`].

use crate::{
    entity::{AnyEntityRef, TrackedEntity},
    error::ActivityError,
};

/// One per caller operation — the workspace-facing request surface of
/// the store server.
pub enum WorkspaceRequest {
    Resolve {
        any_ref: AnyEntityRef,
    },
    /// Check whether an entity exists in the store or substrate.
    /// If found in the substrate, a stub is inserted into the store so subsequent
    /// checks do not re-hit the substrate. Returns `Bool(true/false)`.
    HasRef {
        any_ref: AnyEntityRef,
    },
    Insert {
        entity: TrackedEntity,
    },
    Checkout {
        any_ref: AnyEntityRef,
    },
    Commit {
        entity: TrackedEntity,
    },
    Remove {
        any_ref: AnyEntityRef,
    },
    Persist,
    Load {
        any_ref: AnyEntityRef,
        field: String,
    },
    EnsureMutable {
        any_ref: AnyEntityRef,
        field: String,
    },
    UndoCheckout {
        any_ref: AnyEntityRef,
    },
    /// Roll an entity back to its last persisted state. Removes if newly
    /// added; resets to a stub if it had been committed but not yet
    /// persisted. Requires the entity not be checked out.
    Revert {
        any_ref: AnyEntityRef,
    },
    /// Drop a clean entity's loaded fields, leaving a stub for re-fetch
    /// on next access. Requires the entity not be checked out and have
    /// no pending changes.
    Forget {
        any_ref: AnyEntityRef,
    },
}

/// Typed reply payload. `Err` carries application-level failures
/// from the dispatcher unchanged.
pub enum WorkspaceResponse {
    Entity(TrackedEntity),
    Bool(bool),
    Unit,
    Err(ActivityError),
}
