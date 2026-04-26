//! Workspace-layer methods on a checked-out [`TrackedEntity`].
//!
//! These two methods consume or release the checkout — callers must use
//! one of them to return mutation rights to the store.

use crate::{
    entity::TrackedEntity,
    error::ActivityError,
    store::{StoreRequest, StoreResponse},
    workspace::lib::request::request,
};

impl TrackedEntity {
    /// Validate and merge the checked-out entity back into the store.
    ///
    /// The store runs cross-entity validation against the committed state
    /// before releasing the per-entity lock. Takes the entity by value so
    /// the checked-out handle is consumed on success.
    pub async fn commit(self) -> Result<(), ActivityError> {
        match request(StoreRequest::Commit { entity: self }).await {
            StoreResponse::Unit => Ok(()),
            StoreResponse::Err(e) => Err(e),
            _ => unreachable!(),
        }
    }

    /// Release the checkout without committing any pending changes.
    ///
    /// Drops the caller's edits and returns mutation rights to the store.
    pub async fn undo_checkout(&self) -> Result<(), ActivityError> {
        let any_ref = self.any_ref();
        match request(StoreRequest::UndoCheckout { any_ref }).await {
            StoreResponse::Unit => Ok(()),
            StoreResponse::Err(e) => Err(e),
            _ => unreachable!(),
        }
    }
}
