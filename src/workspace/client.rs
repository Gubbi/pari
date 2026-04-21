use crate::{
    entity::{AnyEntityRef, TrackedEntity},
    store::{StoreRequest, StoreResponse},
    workspace::{
        error::{CheckoutError, CommitError, LoadError, PersistError, ResolveError, UndoError},
        lib::request::request,
    },
};

pub struct EntityClient;

impl EntityClient {
    pub async fn resolve(any_ref: AnyEntityRef) -> Result<TrackedEntity, ResolveError> {
        // TODO: propagate PrimitiveError via ActivityError once the framework exists.
        match request(StoreRequest::Resolve { any_ref })
            .await
            .expect("entity server unavailable")
        {
            StoreResponse::Entity(e) => Ok(e),
            StoreResponse::ResolveErr(e) => Err(e),
            _ => unreachable!(),
        }
    }

    pub async fn insert(entity: TrackedEntity) -> Result<(), CommitError> {
        // TODO: propagate PrimitiveError via ActivityError once the framework exists.
        match request(StoreRequest::Insert { entity })
            .await
            .expect("entity server unavailable")
        {
            StoreResponse::Unit => Ok(()),
            StoreResponse::CommitErr(e) => Err(e),
            _ => unreachable!(),
        }
    }

    pub async fn remove(any_ref: AnyEntityRef) -> TrackedEntity {
        // TODO: propagate PrimitiveError via ActivityError once the framework exists.
        match request(StoreRequest::Remove { any_ref })
            .await
            .expect("entity server unavailable")
        {
            StoreResponse::Entity(e) => e,
            _ => unreachable!(),
        }
    }

    pub async fn checkout(any_ref: AnyEntityRef) -> Result<TrackedEntity, CheckoutError> {
        // TODO: propagate PrimitiveError via ActivityError once the framework exists.
        match request(StoreRequest::Checkout { any_ref })
            .await
            .expect("entity server unavailable")
        {
            StoreResponse::Entity(e) => Ok(e),
            StoreResponse::CheckoutErr(e) => Err(e),
            _ => unreachable!(),
        }
    }

    pub async fn load(any_ref: AnyEntityRef, field: &str) -> Result<(), LoadError> {
        // TODO: propagate PrimitiveError via ActivityError once the framework exists.
        match request(StoreRequest::Load {
            any_ref,
            field: field.to_owned(),
        })
        .await
        .expect("entity server unavailable")
        {
            StoreResponse::Unit => Ok(()),
            StoreResponse::LoadErr(e) => Err(e),
            _ => unreachable!(),
        }
    }

    pub async fn ensure_mutable(any_ref: AnyEntityRef, field: &str) -> Result<(), LoadError> {
        // TODO: propagate PrimitiveError via ActivityError once the framework exists.
        match request(StoreRequest::EnsureMutable {
            any_ref,
            field: field.to_owned(),
        })
        .await
        .expect("entity server unavailable")
        {
            StoreResponse::Unit => Ok(()),
            StoreResponse::LoadErr(e) => Err(e),
            _ => unreachable!(),
        }
    }

    pub async fn persist() -> Result<(), PersistError> {
        // TODO: propagate PrimitiveError via ActivityError once the framework exists.
        match request(StoreRequest::Persist)
            .await
            .expect("entity server unavailable")
        {
            StoreResponse::Unit => Ok(()),
            StoreResponse::PersistErr(e) => Err(e),
            _ => unreachable!(),
        }
    }

    pub async fn undo_commit(any_ref: AnyEntityRef) -> Result<(), UndoError> {
        // TODO: propagate PrimitiveError via ActivityError once the framework exists.
        match request(StoreRequest::UndoCommit { any_ref })
            .await
            .expect("entity server unavailable")
        {
            StoreResponse::Unit => Ok(()),
            StoreResponse::UndoErr(e) => Err(e),
            _ => unreachable!(),
        }
    }

    pub async fn unload(any_ref: AnyEntityRef) -> Result<(), UndoError> {
        // TODO: propagate PrimitiveError via ActivityError once the framework exists.
        match request(StoreRequest::Unload { any_ref })
            .await
            .expect("entity server unavailable")
        {
            StoreResponse::Unit => Ok(()),
            StoreResponse::UndoErr(e) => Err(e),
            _ => unreachable!(),
        }
    }
}
