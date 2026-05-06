//! Substrate parameterization for functional tests.
//!
//! Scenarios where the substrate is incidental to the behavior under
//! test run against both shipped backends via `run_with`. Substrate-
//! specific scenarios pin to a single backend directly using
//! [`with_workspace`].
//!
//! The harness composes a [`Store`], a [`StoreServer`], and a
//! [`Workspace`] for each scenario and drives the actor's run loop
//! inline via `futures::join!`, so tests need no runtime-specific
//! spawner.
//!
//! Drive parameterization with `rstest`:
//!
//! ```ignore
//! #[rstest]
//! #[case::in_memory(SubstrateKind::InMemory)]
//! #[case::repo(SubstrateKind::Repo)]
//! #[tokio::test]
//! async fn scenario(#[case] kind: SubstrateKind) {
//!     run_with(kind, |workspace| async move {
//!         // ... scenario body using workspace.method(...) ...
//!     }).await;
//! }
//! ```

use std::{future::Future, sync::Arc};

use futures::channel::mpsc;
use pari::{
    store::{ChannelStoreDispatcher, Store, StoreDispatcher, StoreServer},
    substrate::{InMemorySubstrate, RepoSubstrate, SchemaBackedSubstrate},
    workspace::Workspace,
};
use tempfile::TempDir;

/// Which backend a scenario should run against.
#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub enum SubstrateKind {
    InMemory,
    Repo,
}

/// Run `scenario` against the substrate identified by `kind`.
///
/// For `Repo`, a fresh tempdir is allocated for the scenario and
/// dropped after it completes.
#[allow(dead_code)]
pub async fn run_with<F, Fut>(kind: SubstrateKind, scenario: F)
where
    F: FnOnce(Workspace) -> Fut + Send,
    Fut: Future<Output = ()> + Send,
{
    match kind {
        SubstrateKind::InMemory => {
            with_workspace(InMemorySubstrate::new(), scenario).await;
        }
        SubstrateKind::Repo => {
            let dir = TempDir::new().expect("create tempdir for RepoSubstrate");
            let substrate =
                RepoSubstrate::new(dir.path().to_path_buf()).expect("construct RepoSubstrate");
            with_workspace(substrate, scenario).await;
            drop(dir);
        }
    }
}

/// Compose `Store` + `StoreServer` + `Workspace` over `substrate` and
/// run `scenario` against the workspace, driving the actor inline via
/// `futures::join!`. Used directly when a scenario needs a specific
/// substrate (e.g. `RepoSubstrate` over a tempdir whose path the test
/// needs to inspect).
#[allow(dead_code)]
pub async fn with_workspace<S, F, Fut>(substrate: S, scenario: F)
where
    S: SchemaBackedSubstrate,
    F: FnOnce(Workspace) -> Fut + Send,
    Fut: Future<Output = ()> + Send,
{
    let (tx, rx) = mpsc::channel(32);
    let store_run = Store::new().run(rx);
    let store_dispatcher: Arc<dyn StoreDispatcher> = Arc::new(ChannelStoreDispatcher::new(tx));
    let server = StoreServer::start(substrate, store_dispatcher);
    let workspace = Workspace::new(server);

    let user_fut = async move {
        scenario(workspace).await;
        // workspace drops here, releasing the StoreServer dispatcher
        // and closing the actor channel; store_run then exits.
    };

    futures::join!(store_run, user_fut);
}
