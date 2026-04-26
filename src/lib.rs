//! Pari — workflow runtime for hybrid human-agent teams.
//!
//! The runtime is organized around the formal `entity`, `workspace`, `store`,
//! `substrate`, `validation`, and `error` layers.
//!
//! Process setup happens through one of two entry points:
//!
//! - [`init`] publishes a process-wide [`EntityServer`] and spawns the
//!   singleton `StoreManager` future via a caller-provided [`SpawnFn`].
//! - [`with`] runs a closure against a thread-local entity server and
//!   drives the manager future internally — used by tests so they need
//!   no runtime-specific spawner.

#![feature(error_generic_member_access)]
#![warn(clippy::pedantic)]
#![allow(clippy::must_use_candidate)]

// Allow `::pari::...` paths in proc-macro-generated code to resolve within
// this crate itself (needed when #[derive(Entity)] is applied inside `pari`).
extern crate self as pari;

use std::{future::Future, sync::Arc};

use futures::{channel::mpsc, future::BoxFuture};

pub mod entity;
pub mod error;
pub use entity::{entities, tracked, types};
pub use error::pari_error::PariError;
pub mod store;
pub mod substrate;
pub mod validation;
pub mod workspace;

use crate::{
    store::{
        install_global_entity_server, install_override_entity_server, EntityServer, StoreManager,
    },
    substrate::SchemaBackedSubstrate,
};

/// Caller-provided spawner used by [`init`] to drive the singleton
/// `StoreManager` future. Production callers wire this to their async
/// runtime of choice (e.g. `tokio::spawn`, `smol::spawn`).
pub type SpawnFn = Arc<dyn Fn(BoxFuture<'static, ()>) + Send + Sync>;

/// Publish a process-wide [`EntityServer`] over `substrate` and spawn
/// the singleton `StoreManager` via `spawn_fn`. Panics if called twice.
///
/// The runtime is not specified by `pari` — `spawn_fn` is the only
/// integration point. Production callers pass a closure that hands the
/// future to their async runtime.
pub fn init<S>(substrate: S, spawn_fn: SpawnFn)
where
    S: SchemaBackedSubstrate,
{
    let (tx, rx) = mpsc::channel(32);
    spawn_fn(Box::pin(StoreManager::new().run(rx)));
    let server: Arc<dyn store::entity_server::Dispatcher> =
        Arc::new(EntityServer::new(substrate, tx));
    install_global_entity_server(server);
}

/// Run `f` against an isolated [`EntityServer`] over `substrate`.
///
/// The thread-local override is installed before `f` runs and torn
/// down after; the singleton `StoreManager` future is driven inside
/// this call via `futures::join!`, so callers do not need a
/// runtime-specific spawner. Multiple `with` calls are isolated from
/// each other and from any process-wide server installed by [`init`].
pub async fn with<S, F, Fut>(substrate: S, f: F)
where
    S: SchemaBackedSubstrate,
    F: FnOnce() -> Fut,
    Fut: Future<Output = ()>,
{
    let (tx, rx) = mpsc::channel(32);
    let manager_fut = StoreManager::new().run(rx);
    let server: Arc<dyn store::entity_server::Dispatcher> =
        Arc::new(EntityServer::new(substrate, tx));

    let user_fut = async move {
        let _guard = install_override_entity_server(server);
        f().await;
        // _guard drops here, releasing the entity-server Arc and closing
        // the manager channel; manager_fut then exits.
    };

    futures::join!(manager_fut, user_fut);
}
