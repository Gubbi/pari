//! Substrate layer — persistence backend trait and implementations.

mod contract;
mod defaults;
pub mod error;
pub mod in_memory;
mod lib;
pub mod repo;
mod tests;
mod void;

pub use contract::Substrate;
pub use error::SubstrateError;
pub use in_memory::{InMemoryStorage, InMemorySubstrate};
pub use lib::pipeline;
pub(crate) use lib::schema_registry::SchemaBackedSubstrate;
pub use repo::RepoSubstrate;
pub use void::VoidSubstrate;
