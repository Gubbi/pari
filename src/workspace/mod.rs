//! Workspace layer — caller-facing entity operations over the store actor.

use crate::entity::{AnyEntityRef, StoreEntity};
use crate::store::{EntityServer, StoreCommand, StoreRequest, StoreResponse};
use crate::store_error::StoreError;

pub mod error;

pub use error::{CheckoutError, CommitError, LoadError, PersistError, ResolveError, UndoError};

pub struct EntityClient;

pub(crate) async fn request(req: StoreRequest) -> Result<StoreResponse, StoreError> {
    EntityServer::request(req).await
}

pub(crate) async fn send(cmd: StoreCommand) -> Result<(), StoreError> {
    EntityServer::send(cmd).await
}

impl EntityClient {
    pub async fn resolve(any_ref: AnyEntityRef) -> Result<StoreEntity, ResolveError> {
        match request(StoreRequest::Resolve { any_ref }).await.map_err(ResolveError::StoreUnavailable)? {
            StoreResponse::Entity(e) => Ok(e),
            StoreResponse::ResolveErr(e) => Err(e),
            _ => unreachable!(),
        }
    }

    pub async fn insert(entity: StoreEntity) -> Result<(), StoreError> {
        send(StoreCommand::Insert(entity)).await
    }

    pub async fn remove(any_ref: AnyEntityRef) -> Result<StoreEntity, StoreError> {
        match request(StoreRequest::Remove { any_ref }).await? {
            StoreResponse::Entity(e) => Ok(e),
            _ => unreachable!(),
        }
    }

    pub async fn checkout(any_ref: AnyEntityRef) -> Result<StoreEntity, CheckoutError> {
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
        match request(StoreRequest::Load { any_ref, field: field.to_owned() })
            .await
            .map_err(LoadError::StoreUnavailable)?
        {
            StoreResponse::Unit => Ok(()),
            StoreResponse::LoadErr(e) => Err(e),
            _ => unreachable!(),
        }
    }

    pub async fn ensure_mutable(any_ref: AnyEntityRef, field: &str) -> Result<(), LoadError> {
        match request(StoreRequest::EnsureMutable { any_ref, field: field.to_owned() })
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
        match request(StoreRequest::UndoCommit { any_ref }).await {
            Ok(StoreResponse::Unit) => Ok(()),
            Ok(_) => unreachable!(),
            Err(e) => Err(UndoError::StoreUnavailable(e)),
        }
    }

    pub async fn unload(any_ref: AnyEntityRef) -> Result<(), UndoError> {
        match request(StoreRequest::Unload { any_ref }).await {
            Ok(StoreResponse::Unit) => Ok(()),
            Ok(_) => unreachable!(),
            Err(e) => Err(UndoError::StoreUnavailable(e)),
        }
    }
}

impl StoreEntity {
    pub async fn commit(self) -> Result<(), CommitError> {
        let any_ref = self.any_ref();
        match request(StoreRequest::Commit { entity: self, any_ref }).await {
            Ok(StoreResponse::Unit) => Ok(()),
            Ok(_) => unreachable!(),
            Err(e) => Err(CommitError::StoreUnavailable(e)),
        }
    }

    pub async fn undo_checkout(&self) -> Result<(), StoreError> {
        let any_ref = self.any_ref();
        send(StoreCommand::UndoCheckout { any_ref }).await
    }
}
