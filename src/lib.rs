//! Pari — workflow runtime for hybrid human-agent teams.
//!
//! Exposes two top-level modules: [`schema`] for entity types and validation,
//! and [`substrate`] for persistence backends.

#![warn(clippy::pedantic)]
#![allow(clippy::must_use_candidate)]

pub mod schema;
pub mod substrate;
