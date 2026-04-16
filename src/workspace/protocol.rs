use crate::store::{EntityServer, StoreRequest, StoreResponse};
use crate::store_error::StoreError;

pub(crate) async fn request(req: StoreRequest) -> Result<StoreResponse, StoreError> {
    EntityServer::request(req).await
}
