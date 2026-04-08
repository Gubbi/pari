//! Pari — workflow runtime for hybrid human-agent teams.
//!
//! Exposes two top-level modules: [`schema`] for entity types and validation,
//! and [`substrate`] for persistence backends.

#![warn(clippy::pedantic)]
#![allow(clippy::must_use_candidate)]

// Allow `::pari::...` paths in proc-macro-generated code to resolve within
// this crate itself (needed when #[derive(Entity)] is applied inside `pari`).
extern crate self as pari;

pub mod entities;
pub mod entity;
pub mod schema;
pub mod store;
pub mod substrate;
pub mod tracked;
pub mod types;
pub mod validation;

#[cfg(test)]
pub mod fixtures;
