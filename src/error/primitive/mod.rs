//! Centralized primitive error repository.
//!
//! Primitive errors are the leaf-most failure evidence in the error model.
//! They live centrally under the formal `error` layer and are organized by
//! shared primitive family rather than by the current owning layer's module
//! layout.

pub mod document;
pub mod io;
pub mod path;
pub mod payload;
pub mod schema;
pub mod state;
pub mod substrate;

pub use document::*;
pub use io::*;
pub use path::*;
pub use payload::*;
pub use schema::*;
pub use state::*;
pub use substrate::*;
