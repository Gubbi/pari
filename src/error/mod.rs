//! Cross-cutting error handling infrastructure.
//!
//! Classification enums, `ErrorCompose` trait, and `OTelEmit` trait.

pub mod activity;
pub mod lib;
pub mod pari_error;
pub mod primitive;

pub use activity::ActivityError;
pub use lib::{
    ErrorCompose, ErrorLayer, ErrorLocation, FixDomain, OTelEmit, PrimitiveDetail, Recoverability,
    Severity,
};
