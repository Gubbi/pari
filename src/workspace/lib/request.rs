//! Pure helper for the workspace → store-server dispatch.
//!
//! Workspace's `client.rs` and the `#[derive(Entity)]`-generated
//! `XDelegate` impls route every caller operation through this single
//! function so the dispatcher lookup lives in one place.

use crate::store::{store_server::active_store_server, WorkspaceRequest, WorkspaceResponse};

/// Dispatch `req` to the active store server and await its reply.
///
/// Application-level failures arrive inside `WorkspaceResponse::Err` and
/// are forwarded unchanged.
pub async fn request(req: WorkspaceRequest) -> WorkspaceResponse {
    active_store_server().dispatch(req).await
}
