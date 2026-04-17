//! EntityStore — async actor-based store for tracked entities.

mod change;
mod message;
mod op_error;
mod server;
mod state;

pub use change::EntityChange;
pub(crate) use message::{StoreRequest, StoreResponse};
pub use op_error::{CheckoutError, CommitError, LoadError, PersistError, ResolveError, UndoError};
pub use server::EntityServer;
