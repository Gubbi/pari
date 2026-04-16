use crate::{
    store::{EntityServer, StoreRequest, StoreResponse},
    store_error::StoreError,
};

pub(crate) async fn request(req: StoreRequest) -> Result<StoreResponse, StoreError> {
    EntityServer::request(req).await
}
