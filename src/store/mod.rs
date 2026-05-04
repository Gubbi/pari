//! Store layer — stateless orchestrator (`StoreServer`) plus state-custodian actor (`Store`).
//!
//! [`StoreServer`] is a stateless dispatcher: workspace calls it
//! directly via the active dispatcher handle, and it sequences
//! substrate calls and validation around requests to the [`Store`]
//! actor (the sole custodian of mutable state).
//!
//! Public surface is intentionally small: process init lives in
//! [`crate::init`] / [`crate::with`], and [`EntityChange`] is the
//! handoff type substrates consume on persist.
//!
//! See `docs/design/layers/store.md` for the L3 design.

mod lib;
pub(crate) mod store;
pub(crate) mod store_server;

pub use lib::{
    change::EntityChange,
    workspace_request::{WorkspaceRequest, WorkspaceResponse},
};
pub(crate) use store_server::{install_global_store_server, install_override_store_server};
