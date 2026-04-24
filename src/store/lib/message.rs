//! Message types for the workspace ↔ `EntityServer` actor channel.
//!
//! Every caller operation from [`workspace`](crate::workspace) maps to
//! exactly one [`StoreRequest`] variant; the server replies with a
//! [`StoreResponse`] carrying either the typed payload or an
//! [`ActivityError`].

use crate::{
    entity::{AnyEntityRef, TrackedEntity},
    error::ActivityError,
};

/// One per caller operation — the workspace-facing request surface of
/// the entity server.
pub(crate) enum StoreRequest {
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
/// across the channel unchanged — channel failure is handled by
/// [`workspace::lib::request`](crate::workspace) wrapping send/recv
/// errors into `ActivityError::store_unavailable`.
pub(crate) enum StoreResponse {
    Entity(TrackedEntity),
    Bool(bool),
    Unit,
    Err(ActivityError),
}

/// Wrapper placed on the server's `mpsc::Receiver`. One-shot reply
/// channel pairs each request with its response.
pub(crate) enum StoreMessage {
    Request {
        request: StoreRequest,
        reply: tokio::sync::oneshot::Sender<StoreResponse>,
    },
}
