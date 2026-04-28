//! Message types for the workspace ↔ `EntityServer` dispatch surface.
//!
//! Every caller operation from [`workspace`](crate::workspace) maps to
//! exactly one [`StoreRequest`] variant; the dispatcher returns a
//! [`StoreResponse`] carrying either the typed payload or an
//! [`ActivityError`].

use crate::{
    entity::{AnyEntityRef, TrackedEntity},
    error::ActivityError,
};

/// One per caller operation — the workspace-facing request surface of
/// the entity server.
pub enum StoreRequest {
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
    UndoCommit {
        any_ref: AnyEntityRef,
    },
    Unload {
        any_ref: AnyEntityRef,
    },
}

/// Typed reply payload. `Err` carries application-level failures
/// from the dispatcher unchanged.
pub enum StoreResponse {
    Entity(TrackedEntity),
    Bool(bool),
    Unit,
    Err(ActivityError),
}
