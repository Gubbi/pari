//! Pari — workflow runtime for hybrid human-agent teams.
//!
//! The runtime is organized around the formal `entity`, `workspace`, `store`,
//! `substrate`, `validation`, and `error` layers.

#![feature(error_generic_member_access)]
#![warn(clippy::pedantic)]
#![allow(clippy::must_use_candidate)]

// Allow `::pari::...` paths in proc-macro-generated code to resolve within
// this crate itself (needed when #[derive(Entity)] is applied inside `pari`).
extern crate self as pari;

pub mod entity;
pub mod error;
pub use entity::{entities, tracked, types};
pub use error::{pari_error::PariError, store as store_error};
pub mod store;
pub mod substrate;
pub mod validation;
pub mod workspace;
