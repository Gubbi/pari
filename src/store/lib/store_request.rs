//! Wire types between [`StoreServer`](super::super::store_server::StoreServer)
//! and the [`Store`](super::super::store::Store) actor.
//!
//! Each [`StoreRequest`] variant maps to one state mutation or query
//! the actor performs; the orchestrator composes these into the
//! caller-facing operations dispatched through
//! [`WorkspaceRequest`](super::workspace_request::WorkspaceRequest).

use futures::channel::oneshot;

use crate::{
    entity::{AnyEntityRef, TrackedEntity},
    error::primitive::PrimitiveError,
    store::lib::change::EntityChange,
};

/// Internal request surface between `StoreServer` and `Store`.
pub(crate) enum StoreRequest {
    // Reads
    GetEntity {
        any_ref: AnyEntityRef,
    },
    ContainsRef {
        any_ref: AnyEntityRef,
    },
    IsFieldLoaded {
        any_ref: AnyEntityRef,
        field: String,
    },
    PendingCheckoutCount,
    // Writes
    InsertStubs {
        refs: Vec<AnyEntityRef>,
    },
    InsertEntity {
        entity: TrackedEntity,
    },
    Checkout {
        any_ref: AnyEntityRef,
    },
    CommitCheckout {
        entity: TrackedEntity,
    },
    UndoCheckout {
        any_ref: AnyEntityRef,
    },
    Revert {
        any_ref: AnyEntityRef,
    },
    RemoveEntity {
        any_ref: AnyEntityRef,
    },
    Forget {
        any_ref: AnyEntityRef,
    },
    InitializeField {
        any_ref: AnyEntityRef,
        loaded: TrackedEntity,
    },
    // Persist lifecycle
    TakePersistSnapshot,
    CommitPersist,
    // State queries
    IsAdded {
        any_ref: AnyEntityRef,
    },
}

pub(crate) enum StoreResponse {
    Entity(TrackedEntity),
    Entities(Vec<TrackedEntity>),
    MaybeEntity(Option<TrackedEntity>),
    Changes(Vec<EntityChange>),
    Bool(bool),
    Count(usize),
    Unit,
    Err(PrimitiveError),
}

pub(crate) struct StoreMessage {
    pub(crate) request: StoreRequest,
    pub(crate) reply: oneshot::Sender<StoreResponse>,
}
