//! EntityStore — async actor-based store for tracked entities.

pub(crate) mod entity_server;
mod lib;
pub(in crate::store) mod manager;

pub use entity_server::EntityServer;
pub use lib::change::EntityChange;
pub(crate) use lib::message::{StoreMessage, StoreRequest, StoreResponse};
