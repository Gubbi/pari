//! Pure support for the validation layer — schema types, the pure
//! runner that accumulates `PrimitiveError` failures, and per-entity
//! rule definitions.

pub mod rules;
pub mod runner;
pub mod schema;
