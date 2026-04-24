//! Pure helper for the workspace → store actor round-trip.
//!
//! Channel send and reply failures are the only [`PrimitiveError`] sources
//! in the workspace layer; this module wraps the mechanics so every caller-
//! facing orchestration site surfaces the same
//! [`ActivityError::store_unavailable`].

use crate::{
    error::{primitive::PrimitiveError, ActivityError},
    store::{entity_server::store_sender, StoreMessage, StoreRequest, StoreResponse},
};

/// Send `req` to the entity server and await its reply.
///
/// Returns the raw [`StoreResponse`] on success. A failed send or a
/// dropped reply becomes `ActivityError::store_unavailable("entity_server", …)`
/// — any application-level error is carried inside `StoreResponse::Err` and
/// forwarded unchanged to the caller.
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
