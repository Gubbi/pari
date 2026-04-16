use crate::entity::TrackedEntity;
use crate::store::{StoreCommand, StoreRequest, StoreResponse};
use crate::store_error::StoreError;
use crate::workspace::error::CommitError;
use crate::workspace::protocol::{request, send};

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

    pub async fn undo_checkout(&self) -> Result<(), StoreError> {
        let any_ref = self.any_ref();
        send(StoreCommand::UndoCheckout { any_ref }).await
    }
}
