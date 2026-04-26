//! Pure helper for the workspace → entity-server dispatch.
//!
//! Workspace's `client.rs` and `tracked_entity.rs` route every caller
//! operation through this single function so the dispatcher lookup
//! lives in one place.

use crate::store::{entity_server::active_entity_server, StoreRequest, StoreResponse};

/// Dispatch `req` to the active entity server and await its reply.
///
/// Application-level failures arrive inside `StoreResponse::Err` and
/// are forwarded unchanged.
pub(crate) async fn request(req: StoreRequest) -> StoreResponse {
    active_entity_server().dispatch(req).await
}
