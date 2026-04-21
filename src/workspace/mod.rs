//! Workspace layer — caller-facing entity operations over the store actor.

mod client;
pub mod error;
mod lib;
mod tracked_entity;

pub use client::EntityClient;
pub use error::{CheckoutError, CommitError, LoadError, PersistError, ResolveError, UndoError};
