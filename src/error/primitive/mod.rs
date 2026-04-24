//! Centralized primitive error repository.
//!
//! The contract for every primitive variant — what fields are fixed, how
//! construction captures diagnostics, how the variant is emitted — lives in
//! [`primitive_errors`]. This module re-exports the generated enum and the
//! shared `PrimitiveContext` that carries the fixed diagnostics.

mod primitive_errors;

pub use primitive_errors::{PrimitiveContext, PrimitiveError};
