//! Persistence contract and schema-driven backends.
//!
//! The [`Substrate`] trait is the single surface the `store` layer calls
//! during resolve, load, ensure_mutable, and persist. Its four data
//! methods default to a schema-driven pipeline composed of three
//! backend-supplied components — a [`pipeline::LocationResolver`], a
//! [`pipeline::Codec`], and a [`pipeline::Executor`] — so backends
//! typically only implement the trait surface and delegate to the
//! defaults.
//!
//! Three concrete backends ship with the crate:
//! [`RepoSubstrate`] (filesystem), [`InMemorySubstrate`] (RAM), and
//! [`VoidSubstrate`] (no-op for test scaffolding).
//!
//! See `docs/design/layers/substrate.md` for the L3 design.

mod defaults;
pub mod in_memory;
mod lib;
pub mod repo;
mod substrate;
mod void;

pub use in_memory::{InMemoryStorage, InMemorySubstrate};
pub use lib::pipeline;
pub(crate) use lib::schema_registry::SchemaBackedSubstrate;
pub use repo::RepoSubstrate;
pub use substrate::Substrate;
pub use void::VoidSubstrate;
