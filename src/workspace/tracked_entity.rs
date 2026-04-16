use crate::{
    entity::TrackedEntity,
    store::{StoreRequest, StoreResponse},
    workspace::{
        error::{CommitError, UndoError},
        protocol::request,
    },
};

impl TrackedEntity {
    pub async fn commit(self) -> Result<(), CommitError> {
        match request(StoreRequest::Commit { entity: self })
            .await
            .map_err(CommitError::StoreUnavailable)?
        {
            StoreResponse::Unit => Ok(()),
            StoreResponse::CommitErr(e) => Err(e),
            _ => unreachable!(),
        }
    }

    pub async fn undo_checkout(&self) -> Result<(), UndoError> {
        let any_ref = self.any_ref();
        match request(StoreRequest::UndoCheckout { any_ref })
            .await
            .map_err(UndoError::StoreUnavailable)?
        {
            StoreResponse::Unit => Ok(()),
            StoreResponse::UndoErr(e) => Err(e),
            _ => unreachable!(),
        }
    }
}
