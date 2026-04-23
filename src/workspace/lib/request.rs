use crate::{
    error::{primitive::PrimitiveError, ActivityError},
    store::{entity_server::store_sender, StoreMessage, StoreRequest, StoreResponse},
};

pub(crate) async fn request(req: StoreRequest) -> Result<StoreResponse, ActivityError> {
    let (tx, rx) = tokio::sync::oneshot::channel();
    store_sender()
        .send(StoreMessage::Request {
            request: req,
            reply: tx,
        })
        .await
        .map_err(|_| {
            ActivityError::store_unavailable(
                "entity_server",
                PrimitiveError::store_unavailable("entity server send failed"),
            )
        })?;
    rx.await.map_err(|_| {
        ActivityError::store_unavailable(
            "entity_server",
            PrimitiveError::store_unavailable("entity server reply dropped"),
        )
    })
}
