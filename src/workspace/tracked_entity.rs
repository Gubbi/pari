use crate::{
    entity::TrackedEntity,
    store::{StoreRequest, StoreResponse},
    workspace::{
        error::{CommitError, UndoError},
        lib::request::request,
    },
};

impl TrackedEntity {
    pub async fn commit(self) -> Result<(), CommitError> {
        // TODO: propagate PrimitiveError via ActivityError once the framework exists.
        match request(StoreRequest::Commit { entity: self })
            .await
            .expect("entity server unavailable")
        {
            StoreResponse::Unit => Ok(()),
            StoreResponse::CommitErr(e) => Err(e),
            _ => unreachable!(),
        }
    }

    pub async fn undo_checkout(&self) -> Result<(), UndoError> {
        // TODO: propagate PrimitiveError via ActivityError once the framework exists.
        let any_ref = self.any_ref();
        match request(StoreRequest::UndoCheckout { any_ref })
            .await
            .expect("entity server unavailable")
        {
            StoreResponse::Unit => Ok(()),
            StoreResponse::UndoErr(e) => Err(e),
            _ => unreachable!(),
        }
    }
}
