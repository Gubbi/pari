use crate::{
    entity::{AnyEntityRef, TrackedEntity},
    store::{StoreRequest, StoreResponse},
    store_error::StoreError,
    workspace::{
        error::{CheckoutError, CommitError, LoadError, PersistError, ResolveError, UndoError},
        protocol::request,
    },
};

pub struct EntityClient;

impl EntityClient {
    pub async fn resolve(any_ref: AnyEntityRef) -> Result<TrackedEntity, ResolveError> {
        match request(StoreRequest::Resolve { any_ref })
            .await
            .map_err(ResolveError::StoreUnavailable)?
        {
            StoreResponse::Entity(e) => Ok(e),
            StoreResponse::ResolveErr(e) => Err(e),
            _ => unreachable!(),
        }
    }

    pub async fn insert(entity: TrackedEntity) -> Result<(), CommitError> {
        match request(StoreRequest::Insert { entity })
            .await
            .map_err(CommitError::StoreUnavailable)?
        {
            StoreResponse::Unit => Ok(()),
            StoreResponse::CommitErr(e) => Err(e),
            _ => unreachable!(),
        }
    }

    pub async fn remove(any_ref: AnyEntityRef) -> Result<TrackedEntity, StoreError> {
        match request(StoreRequest::Remove { any_ref }).await? {
            StoreResponse::Entity(e) => Ok(e),
            _ => unreachable!(),
        }
    }

    pub async fn checkout(any_ref: AnyEntityRef) -> Result<TrackedEntity, CheckoutError> {
        match request(StoreRequest::Checkout { any_ref })
            .await
            .map_err(CheckoutError::StoreUnavailable)?
        {
            StoreResponse::Entity(e) => Ok(e),
            StoreResponse::CheckoutErr(e) => Err(e),
            _ => unreachable!(),
        }
    }

    pub async fn load(any_ref: AnyEntityRef, field: &str) -> Result<(), LoadError> {
        match request(StoreRequest::Load {
            any_ref,
            field: field.to_owned(),
        })
        .await
        .map_err(LoadError::StoreUnavailable)?
        {
            StoreResponse::Unit => Ok(()),
            StoreResponse::LoadErr(e) => Err(e),
            _ => unreachable!(),
        }
    }

    pub async fn ensure_mutable(any_ref: AnyEntityRef, field: &str) -> Result<(), LoadError> {
        match request(StoreRequest::EnsureMutable {
            any_ref,
            field: field.to_owned(),
        })
        .await
        .map_err(LoadError::StoreUnavailable)?
        {
            StoreResponse::Unit => Ok(()),
            StoreResponse::LoadErr(e) => Err(e),
            _ => unreachable!(),
        }
    }

    pub async fn persist() -> Result<(), PersistError> {
        match request(StoreRequest::Persist)
            .await
            .map_err(PersistError::StoreUnavailable)?
        {
            StoreResponse::Unit => Ok(()),
            StoreResponse::PersistErr(e) => Err(e),
            _ => unreachable!(),
        }
    }

    pub async fn undo_commit(any_ref: AnyEntityRef) -> Result<(), UndoError> {
        match request(StoreRequest::UndoCommit { any_ref })
            .await
            .map_err(UndoError::StoreUnavailable)?
        {
            StoreResponse::Unit => Ok(()),
            StoreResponse::UndoErr(e) => Err(e),
            _ => unreachable!(),
        }
    }

    pub async fn unload(any_ref: AnyEntityRef) -> Result<(), UndoError> {
        match request(StoreRequest::Unload { any_ref })
            .await
            .map_err(UndoError::StoreUnavailable)?
        {
            StoreResponse::Unit => Ok(()),
            StoreResponse::UndoErr(e) => Err(e),
            _ => unreachable!(),
        }
    }
}
