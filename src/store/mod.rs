//! Store layer — stateless orchestrator (`EntityServer`) plus state-custodian actor (`StoreManager`).
//!
//! [`EntityServer`] is a stateless dispatcher: workspace calls it
//! directly via the active dispatcher handle, and it sequences
//! substrate calls and validation around requests to the singleton
//! `StoreManager` (the sole custodian of mutable state).
//!
//! Public surface is intentionally small: process init lives in
//! [`crate::init`] / [`crate::with`], and [`EntityChange`] is the
//! handoff type substrates consume on persist.
//!
//! See `docs/design/layers/store.md` for the L3 design.

pub(crate) mod entity_server;
mod lib;
pub(crate) mod manager;

pub(crate) use entity_server::{
    install_global_entity_server, install_override_entity_server, EntityServer,
};
pub use lib::change::EntityChange;
pub(crate) use lib::message::{StoreRequest, StoreResponse};
pub(crate) use manager::StoreManager;
