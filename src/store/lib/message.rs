use crate::{
    entity::{AnyEntityRef, TrackedEntity},
    error::ActivityError,
};

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

pub(crate) enum StoreResponse {
    Entity(TrackedEntity),
    Bool(bool),
    Unit,
    Err(ActivityError),
}

pub(crate) enum StoreMessage {
    Request {
        request: StoreRequest,
        reply: tokio::sync::oneshot::Sender<StoreResponse>,
    },
}
