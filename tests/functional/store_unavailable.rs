//! User job: workspace ↔ store boundary failures surface as
//! `ActivityError::StoreUnavailable`, never as panics or silent
//! success.
//!
//! Every workspace operation reaches the store through `store_send`,
//! which classifies channel-level transport faults as
//! `ActivityError::store_unavailable`. The tests here drive that
//! boundary with two harnesses:
//!
//! - `BrokenStoreDispatcher` — every dispatch returns
//!   `PrimitiveError::store_unavailable`. Stands in for a Store
//!   actor that has terminated before the workspace tried to talk
//!   to it.
//! - `ToggleStoreDispatcher` — wraps a real dispatcher; an
//!   `AtomicBool` flag makes it start failing on demand. Stands in
//!   for the actor terminating *during* a session, after some setup
//!   has succeeded.

use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use futures::{channel::mpsc, future::BoxFuture, FutureExt};
use pari::{
    entities::role::Role,
    error::{primitive::PrimitiveError, ActivityError},
    store::{
        ChannelStoreDispatcher, Dispatcher, Store, StoreDispatcher, StoreRequest, StoreResponse,
        StoreServer,
    },
    substrate::InMemorySubstrate,
    workspace::Workspace,
};

use crate::fixtures::role::a_minimal_role;

// ---------------------------------------------------------------------------
// Harnesses
// ---------------------------------------------------------------------------

struct BrokenStoreDispatcher;

impl StoreDispatcher for BrokenStoreDispatcher {
    fn dispatch<'a>(
        &'a self,
        _req: StoreRequest,
    ) -> BoxFuture<'a, Result<StoreResponse, PrimitiveError>> {
        async { Err(PrimitiveError::store_unavailable("store unavailable")) }.boxed()
    }
}

struct ToggleStoreDispatcher {
    inner: Arc<dyn StoreDispatcher>,
    broken: Arc<AtomicBool>,
}

impl StoreDispatcher for ToggleStoreDispatcher {
    fn dispatch<'a>(
        &'a self,
        req: StoreRequest,
    ) -> BoxFuture<'a, Result<StoreResponse, PrimitiveError>> {
        let inner = Arc::clone(&self.inner);
        let broken = Arc::clone(&self.broken);
        async move {
            if broken.load(Ordering::SeqCst) {
                return Err(PrimitiveError::store_unavailable("store unavailable"));
            }
            inner.dispatch(req).await
        }
        .boxed()
    }
}

fn workspace_with_broken_store() -> (Workspace, BoxFuture<'static, ()>) {
    let server: Arc<dyn Dispatcher> =
        StoreServer::start(InMemorySubstrate::new(), Arc::new(BrokenStoreDispatcher));
    let workspace = Workspace::new(server);
    // Broken dispatcher has no actor to drive; return a no-op future
    // so callers can still `futures::join!` for symmetry with the
    // real harness.
    (workspace, async {}.boxed())
}

fn workspace_with_toggle_store() -> (Workspace, BoxFuture<'static, ()>, Arc<AtomicBool>) {
    let (tx, rx) = mpsc::channel(32);
    let store_run = Store::new().run(rx).boxed();
    let inner: Arc<dyn StoreDispatcher> = Arc::new(ChannelStoreDispatcher::new(tx));
    let broken = Arc::new(AtomicBool::new(false));
    let dispatcher: Arc<dyn StoreDispatcher> = Arc::new(ToggleStoreDispatcher {
        inner,
        broken: Arc::clone(&broken),
    });
    let server = StoreServer::start(InMemorySubstrate::new(), dispatcher);
    let workspace = Workspace::new(server);
    (workspace, store_run, broken)
}

fn assert_store_unavailable<T>(result: Result<T, ActivityError>) {
    let err = match result {
        Ok(_) => panic!("expected StoreUnavailable, got Ok"),
        Err(e) => e,
    };
    assert!(
        matches!(err, ActivityError::StoreUnavailable { .. }),
        "expected StoreUnavailable, got: {err:?}"
    );
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[tokio::test]
async fn resolve_with_unavailable_store_surfaces_store_unavailable() {
    let (workspace, store_run) = workspace_with_broken_store();
    let scenario = async move {
        assert_store_unavailable(workspace.resolve(role_ref("eng-lead")).await);
    };
    futures::join!(store_run, scenario);
}

#[tokio::test]
async fn persist_with_unavailable_store_surfaces_store_unavailable() {
    let (workspace, store_run) = workspace_with_broken_store();
    let scenario = async move {
        // Persist's first store interaction is `PendingCheckoutCount` —
        // it never reaches the substrate. Surfaces store_unavailable
        // before any side effects.
        assert_store_unavailable(workspace.persist().await);
    };
    futures::join!(store_run, scenario);
}

#[tokio::test]
async fn field_load_after_store_drops_surfaces_store_unavailable() {
    let (workspace, store_run, broken) = workspace_with_toggle_store();
    let scenario = async move {
        // Setup phase: real dispatcher, normal flow.
        workspace.insert(a_minimal_role("eng-lead")).await.unwrap();
        workspace.persist().await.unwrap();
        workspace.forget(role_ref("eng-lead")).await.unwrap();

        // Resolve still works — it hits the store before the field
        // accessor does, so the viewer is bound to a stub entity.
        let role = workspace.resolve(role_ref("eng-lead")).await.unwrap();

        // Simulate the actor terminating mid-session.
        broken.store(true, Ordering::SeqCst);

        // Accessor triggers Load → store_send → broken dispatcher.
        assert_store_unavailable(role.name().await);

        // Drop workspace so the inner dispatcher's tx closes and the
        // store_run future can complete.
        drop(role);
        drop(workspace);
    };
    futures::join!(store_run, scenario);
}

fn role_ref(id: &str) -> pari::entity::EntityRef<Role> {
    pari::entity::EntityRef::new(id)
}
