//! Store layer — stateless orchestrator (`StoreServer`) plus state-custodian actor (`Store`).
//!
//! [`StoreServer`] is a stateless dispatcher: workspace calls it
//! through the [`Dispatcher`] trait, and it sequences substrate calls
//! and validation around requests to the [`Store`] actor (the sole
//! custodian of mutable state).
//!
//! Components are constructed bottom-up:
//!
//! - [`Store::start(spawn_fn)`](Store::start) returns
//!   `Arc<dyn StoreDispatcher>`.
//! - [`StoreServer::start(substrate, store_dispatcher)`](StoreServer::start)
//!   returns `Arc<dyn Dispatcher>`.
//! - That dispatcher is what `Workspace::new` consumes.
//!
//! See `docs/design/layers/store.md` for the L3 design.

mod lib;
pub mod store;
pub mod store_server;

pub use lib::{
    change::EntityChange,
    store_request::{StoreMessage, StoreRequest, StoreResponse},
    workspace_request::{WorkspaceRequest, WorkspaceResponse},
};
pub use store::{ChannelStoreDispatcher, Store, StoreDispatcher};
pub use store_server::{Dispatcher, StoreServer};
