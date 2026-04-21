//! Cross-cutting error handling infrastructure.
//!
//! Classification enums, `ErrorCompose` trait, `OTelEmit` trait, and `BatchError<E>`.

pub mod lib;
pub mod pari_error;
pub mod primitive;
pub mod store;

pub use lib::{
    ActivityComponent, BatchError, ErrorCompose, ErrorLayer, ErrorLocation, FixDomain, OTelEmit,
    PrimitiveDetail, Recoverability, Severity,
};
