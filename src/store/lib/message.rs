use crate::{
    entity::{AnyEntityRef, TrackedEntity},
    error::store::StoreError,
    store::lib::op_error::{
        CheckoutError, CommitError, LoadError, PersistError, ResolveError, UndoError,
    },
};

pub(crate) enum StoreRequest {
    Resolve {
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
    Unit,
    ResolveErr(ResolveError),
    CommitErr(CommitError),
    CheckoutErr(CheckoutError),
    PersistErr(PersistError),
    LoadErr(LoadError),
    UndoErr(UndoError),
}

pub(in crate::store) enum StoreMessage {
    Request {
        request: StoreRequest,
        reply: tokio::sync::oneshot::Sender<Result<StoreResponse, StoreError>>,
    },
}
