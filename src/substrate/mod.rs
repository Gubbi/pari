//! Substrate layer — persistence backend trait and implementations.

mod contract;
mod defaults;
pub mod error;
pub mod in_memory;
pub mod pipeline;
pub mod repo;
pub(crate) mod schema_registry;
mod serde;
mod tests;
mod void;

pub use contract::Substrate;
pub use error::SubstrateError;
pub use in_memory::{InMemoryStorage, InMemorySubstrate};
pub use repo::RepoSubstrate;
pub use void::VoidSubstrate;
