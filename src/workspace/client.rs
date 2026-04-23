use crate::{
    entity::{AnyEntityRef, TrackedEntity},
    error::ActivityError,
    store::{StoreRequest, StoreResponse},
    workspace::lib::request::request,
};

pub struct EntityClient;

impl EntityClient {
    pub async fn resolve(any_ref: AnyEntityRef) -> Result<TrackedEntity, ActivityError> {
        match request(StoreRequest::Resolve { any_ref }).await? {
            StoreResponse::Entity(e) => Ok(e),
            StoreResponse::Err(e) => Err(e),
            _ => unreachable!(),
        }
    }

    /// Returns `true` if the ref exists in the store (or substrate), `false` if not.
    /// Inserts a stub so subsequent checks avoid re-hitting the substrate.
    pub async fn has_ref(any_ref: AnyEntityRef) -> Result<bool, ActivityError> {
        match request(StoreRequest::HasRef { any_ref }).await? {
            StoreResponse::Bool(b) => Ok(b),
            StoreResponse::Err(e) => Err(e),
            _ => unreachable!(),
        }
    }

    pub async fn insert(entity: TrackedEntity) -> Result<(), ActivityError> {
        match request(StoreRequest::Insert { entity }).await? {
            StoreResponse::Unit => Ok(()),
            StoreResponse::Err(e) => Err(e),
            _ => unreachable!(),
        }
    }

    pub async fn remove(any_ref: AnyEntityRef) -> Result<TrackedEntity, ActivityError> {
        match request(StoreRequest::Remove { any_ref }).await? {
            StoreResponse::Entity(e) => Ok(e),
            StoreResponse::Err(e) => Err(e),
            _ => unreachable!(),
        }
    }

    pub async fn checkout(any_ref: AnyEntityRef) -> Result<TrackedEntity, ActivityError> {
        match request(StoreRequest::Checkout { any_ref }).await? {
            StoreResponse::Entity(e) => Ok(e),
            StoreResponse::Err(e) => Err(e),
            _ => unreachable!(),
        }
    }

    pub async fn load(any_ref: AnyEntityRef, field: &str) -> Result<(), ActivityError> {
        match request(StoreRequest::Load {
            any_ref,
            field: field.to_owned(),
        })
        .await?
        {
            StoreResponse::Unit => Ok(()),
            StoreResponse::Err(e) => Err(e),
            _ => unreachable!(),
        }
    }

    pub async fn ensure_mutable(any_ref: AnyEntityRef, field: &str) -> Result<(), ActivityError> {
        match request(StoreRequest::EnsureMutable {
            any_ref,
            field: field.to_owned(),
        })
        .await?
        {
            StoreResponse::Unit => Ok(()),
            StoreResponse::Err(e) => Err(e),
            _ => unreachable!(),
        }
    }

    pub async fn persist() -> Result<(), ActivityError> {
        match request(StoreRequest::Persist).await? {
            StoreResponse::Unit => Ok(()),
            StoreResponse::Err(e) => Err(e),
            _ => unreachable!(),
        }
    }

    pub async fn undo_commit(any_ref: AnyEntityRef) -> Result<(), ActivityError> {
        match request(StoreRequest::UndoCommit { any_ref }).await? {
            StoreResponse::Unit => Ok(()),
            StoreResponse::Err(e) => Err(e),
            _ => unreachable!(),
        }
    }

    pub async fn unload(any_ref: AnyEntityRef) -> Result<(), ActivityError> {
        match request(StoreRequest::Unload { any_ref }).await? {
            StoreResponse::Unit => Ok(()),
            StoreResponse::Err(e) => Err(e),
            _ => unreachable!(),
        }
    }
}
