//! Async actor-based store for tracked entities.
//!
//! Split into two actors: [`EntityServer`] owns orchestration (substrate
//! calls, validation sequencing, load/persist flow), and the internal
//! `StoreManager` is the sole custodian of mutable state. Workspace
//! forwards every caller request here; callers never touch the store
//! directly.
//!
//! Public surface is intentionally small: [`EntityServer`] for process
//! init and scoped test setup, and [`EntityChange`] as the handoff type
//! substrates consume on persist.
//!
//! See `docs/design/layers/store.md` for the L3 design.

pub(crate) mod entity_server;
mod lib;
pub(in crate::store) mod manager;

pub use entity_server::EntityServer;
pub use lib::change::EntityChange;
pub(crate) use lib::message::{StoreMessage, StoreRequest, StoreResponse};
