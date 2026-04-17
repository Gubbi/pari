use crate::{
    error::store::StoreError,
    store::{EntityServer, StoreRequest, StoreResponse},
};

pub(crate) async fn request(req: StoreRequest) -> Result<StoreResponse, StoreError> {
    EntityServer::request(req).await
}
