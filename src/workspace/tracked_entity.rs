use crate::{
    entity::TrackedEntity,
    error::ActivityError,
    store::{StoreRequest, StoreResponse},
    workspace::lib::request::request,
};

impl TrackedEntity {
    pub async fn commit(self) -> Result<(), ActivityError> {
        match request(StoreRequest::Commit { entity: self }).await? {
            StoreResponse::Unit => Ok(()),
            StoreResponse::Err(e) => Err(e),
            _ => unreachable!(),
        }
    }

    pub async fn undo_checkout(&self) -> Result<(), ActivityError> {
        let any_ref = self.any_ref();
        match request(StoreRequest::UndoCheckout { any_ref }).await? {
            StoreResponse::Unit => Ok(()),
            StoreResponse::Err(e) => Err(e),
            _ => unreachable!(),
        }
    }
}
