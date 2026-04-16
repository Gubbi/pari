use crate::store::{EntityServer, StoreCommand, StoreRequest, StoreResponse};
use crate::store_error::StoreError;

pub(crate) async fn request(req: StoreRequest) -> Result<StoreResponse, StoreError> {
    EntityServer::request(req).await
}

pub(crate) async fn send(cmd: StoreCommand) -> Result<(), StoreError> {
    EntityServer::send(cmd).await
}
