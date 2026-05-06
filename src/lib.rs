//! Pari — workflow runtime for hybrid human-agent teams.
//!
//! The runtime is organized around the formal `entity`, `workspace`,
//! `store`, `substrate`, and `error` layers.
//!
//! Integrators compose Pari from the bottom up:
//!
//! ```ignore
//! let store_dispatcher  = pari::store::Store::start(&spawn_fn);
//! let server_dispatcher = pari::store::StoreServer::start(substrate, store_dispatcher);
//! let workspace         = pari::workspace::Workspace::new(server_dispatcher);
//! ```
//!
//! There are no globals. The runtime is the integrator's choice —
//! [`SpawnFn`] is the only async-runtime touch-point inside `pari`.

#![feature(error_generic_member_access)]
#![warn(clippy::pedantic)]
#![allow(clippy::must_use_candidate)]

// Allow `::pari::...` paths in proc-macro-generated code to resolve within
// this crate itself (needed when #[derive(Entity)] is applied inside `pari`).
extern crate self as pari;

use std::sync::Arc;

use futures::future::BoxFuture;

pub mod entity;
pub mod error;
pub use entity::{entities, tracked, types};
pub use error::pari_error::PariError;
pub mod store;
pub mod substrate;
pub mod workspace;
/// Validation lives inside the `workspace` layer; this re-export keeps
/// the `pari::validation::*` paths used by `#[derive(Entity)]`-generated
/// code and external callers stable across the relocation.
pub use workspace::validation;

/// Caller-provided spawner used by [`store::Store::start`] to drive
/// the [`store::Store`] actor future. Production callers wire this to
/// their async runtime of choice (e.g. `tokio::spawn`, `smol::spawn`).
pub type SpawnFn = Arc<dyn Fn(BoxFuture<'static, ()>) + Send + Sync>;
