//! Workspace layer — caller-facing entity operations over the store actor.

mod client;
pub mod error;
mod protocol;
mod tracked_entity;

pub use client::EntityClient;
pub use error::{CheckoutError, CommitError, LoadError, PersistError, ResolveError, UndoError};
