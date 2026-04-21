//! EntityStore — async actor-based store for tracked entities.

mod lib;
mod server;
mod state;

pub use lib::{
    change::EntityChange,
    op_error::{CheckoutError, CommitError, LoadError, PersistError, ResolveError, UndoError},
};
pub(crate) use lib::{
    change::PersistChanges,
    message::{StoreRequest, StoreResponse},
};
pub use server::EntityServer;
