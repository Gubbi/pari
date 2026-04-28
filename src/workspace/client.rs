//! [`EntityClient`] — typed wrapper over the entity-server dispatch surface.
//!
//! Every method builds one [`StoreRequest`], dispatches it through the
//! active entity server, and returns the typed result. Application-level
//! failures arrive inside `StoreResponse::Err` and are forwarded
//! unchanged.

use crate::{
    entity::{AnyEntityRef, Entity, EntityRef, TrackedEntity},
    error::ActivityError,
    store::{StoreRequest, StoreResponse},
    workspace::lib::request::request,
};

/// Zero-sized handle for issuing store operations.
///
/// There is no client state — each call takes the `AnyEntityRef` (or other
/// inputs) it needs and dispatches through the active entity server.
/// Methods are all `async fn` and return `Result<_, ActivityError>`.
pub struct EntityClient;

impl EntityClient {
    /// Fetch a snapshot of the entity at `any_ref`.
    ///
    /// The returned [`TrackedEntity`] may be a stub — existence has been
    /// confirmed but no fields are necessarily loaded. Subsequent accessor
    /// calls trigger transparent loads on demand.
    pub async fn resolve(any_ref: AnyEntityRef) -> Result<TrackedEntity, ActivityError> {
        match request(StoreRequest::Resolve { any_ref }).await {
            StoreResponse::Entity(e) => Ok(e),
            StoreResponse::Err(e) => Err(e),
            _ => unreachable!(),
        }
    }

    /// Confirm whether an entity exists at `any_ref`.
    ///
    /// Returns `true` if found in the store or (on a miss) the substrate;
    /// returns `false` otherwise. On a confirmed hit a stub is inserted
    /// into the store so later lookups avoid the substrate round-trip.
    /// This is the pathway validators use for cross-entity existence
    /// checks.
    pub async fn has_ref(any_ref: AnyEntityRef) -> Result<bool, ActivityError> {
        match request(StoreRequest::HasRef { any_ref }).await {
            StoreResponse::Bool(b) => Ok(b),
            StoreResponse::Err(e) => Err(e),
            _ => unreachable!(),
        }
    }

    /// Add a new entity to the store.
    ///
    /// Fails if an entity with the same ref already exists.
    pub async fn insert(entity: TrackedEntity) -> Result<(), ActivityError> {
        match request(StoreRequest::Insert { entity }).await {
            StoreResponse::Unit => Ok(()),
            StoreResponse::Err(e) => Err(e),
            _ => unreachable!(),
        }
    }

    /// Evict an entity from the store.
    ///
    /// Returns the removed [`TrackedEntity`] — pass it back to [`Self::insert`]
    /// to undo the removal.
    pub async fn remove(any_ref: AnyEntityRef) -> Result<TrackedEntity, ActivityError> {
        match request(StoreRequest::Remove { any_ref }).await {
            StoreResponse::Entity(e) => Ok(e),
            StoreResponse::Err(e) => Err(e),
            _ => unreachable!(),
        }
    }

    /// Acquire per-entity exclusive mutation rights.
    ///
    /// Returns the typed [`Entity::Delegate`] for `T`, which exposes
    /// setters and the `commit` / `undo_checkout` lifecycle. The
    /// delegate is not [`Clone`] and consumes itself on release.
    /// Subsequent checkout attempts for the same ref fail until the
    /// active delegate is committed or undone.
    pub async fn checkout<T: Entity>(
        entity_ref: EntityRef<T, T::Parent>,
    ) -> Result<T::Delegate, ActivityError>
    where
        T::Delegate: From<T::Tracked>,
    {
        let any_ref = T::to_any_ref(&entity_ref);
        let entity = match request(StoreRequest::Checkout { any_ref }).await {
            StoreResponse::Entity(e) => e,
            StoreResponse::Err(e) => return Err(e),
            _ => unreachable!(),
        };
        let tracked = T::take(entity)
            .unwrap_or_else(|_| unreachable!("store returned the wrong tracked variant for T"));
        Ok(T::Delegate::from(tracked))
    }

    /// Explicitly load a field.
    ///
    /// Generated accessors call this transparently on first access; direct
    /// use is rare and mainly appears in the progressive-load loop and in
    /// validation-driven ref resolution.
    pub async fn load(any_ref: AnyEntityRef, field: &str) -> Result<(), ActivityError> {
        match request(StoreRequest::Load {
            any_ref,
            field: field.to_owned(),
        })
        .await
        {
            StoreResponse::Unit => Ok(()),
            StoreResponse::Err(e) => Err(e),
            _ => unreachable!(),
        }
    }

    /// Prepare `field` for overwrite.
    ///
    /// Called by generated setters before the candidate swap. The store
    /// loads any declared prerequisites and, if the substrate requires it,
    /// the field itself — so a later load cannot silently clobber the
    /// pending mutation. Direct use outside generated code is rare.
    pub async fn ensure_mutable(any_ref: AnyEntityRef, field: &str) -> Result<(), ActivityError> {
        match request(StoreRequest::EnsureMutable {
            any_ref,
            field: field.to_owned(),
        })
        .await
        {
            StoreResponse::Unit => Ok(()),
            StoreResponse::Err(e) => Err(e),
            _ => unreachable!(),
        }
    }

    /// Flush the store's pending changes to the substrate.
    ///
    /// Fails if any entity is currently checked out — callers must either
    /// commit or undo every checkout first.
    pub async fn persist() -> Result<(), ActivityError> {
        match request(StoreRequest::Persist).await {
            StoreResponse::Unit => Ok(()),
            StoreResponse::Err(e) => Err(e),
            _ => unreachable!(),
        }
    }

    /// Revert the entity to its last persisted state.
    ///
    /// Removes the entity if it was freshly added; resets it to a stub if
    /// it had been committed but not yet persisted. Requires the entity
    /// not be checked out.
    pub async fn undo_commit(any_ref: AnyEntityRef) -> Result<(), ActivityError> {
        match request(StoreRequest::UndoCommit { any_ref }).await {
            StoreResponse::Unit => Ok(()),
            StoreResponse::Err(e) => Err(e),
            _ => unreachable!(),
        }
    }

    /// Reset a clean entity back to a stub.
    ///
    /// Drops loaded fields so the next accessor triggers a fresh fetch.
    /// Requires the entity not be checked out and have no pending changes.
    pub async fn unload(any_ref: AnyEntityRef) -> Result<(), ActivityError> {
        match request(StoreRequest::Unload { any_ref }).await {
            StoreResponse::Unit => Ok(()),
            StoreResponse::Err(e) => Err(e),
            _ => unreachable!(),
        }
    }
}
