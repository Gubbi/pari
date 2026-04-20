//! Centralized primitive error repository.
//!
//! Primitive errors are the leaf-most failure evidence in the error model.

mod primitive_errors;

pub use primitive_errors::{PrimitiveContext, PrimitiveError};
