//! Substrate layer — persistence backend trait and implementations.

mod defaults;
pub mod in_memory;
mod lib;
pub mod repo;
mod substrate;
mod tests;
mod void;

pub use in_memory::{InMemoryStorage, InMemorySubstrate};
pub use lib::pipeline;
pub(crate) use lib::schema_registry::SchemaBackedSubstrate;
pub use repo::RepoSubstrate;
pub use substrate::Substrate;
pub use void::VoidSubstrate;
