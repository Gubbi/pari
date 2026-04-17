//! Workspace-layer re-exports of store-owned operation errors.

pub use crate::store::{
    CheckoutError, CommitError, LoadError, PersistError, ResolveError, UndoError,
};
