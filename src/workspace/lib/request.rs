use crate::{
    error::primitive::PrimitiveError,
    store::{EntityServer, StoreRequest, StoreResponse},
};

pub(crate) async fn request(req: StoreRequest) -> Result<StoreResponse, PrimitiveError> {
    EntityServer::request(req)
        .await
        .map_err(|_| PrimitiveError::store_unavailable("entity server actor unreachable"))
}
