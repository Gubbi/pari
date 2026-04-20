//! Substrate layer — persistence backend trait and implementations.

mod contract;
mod defaults;
pub mod error;
pub mod in_memory;
pub mod pipeline;
pub mod repo;
pub(crate) mod schema_registry;
mod serde;
mod void;

pub use contract::Substrate;
pub use error::SubstrateError;
pub use in_memory::{InMemoryStorage, InMemorySubstrate};
pub use repo::RepoSubstrate;
pub use void::VoidSubstrate;

#[cfg(test)]
mod tests {
    use crate::{error::primitive, substrate::SubstrateError};

    #[test]
    fn substrate_error_display_format() {
        let primitive = primitive::PrimitiveError::PathPermissionDenied {
            context: primitive::PrimitiveError::context("path permission denied"),
            asset_path: "roles/eng-lead.md".to_string(),
            operation: "get".to_string(),
        };
        let error = SubstrateError::corrupt_persistence_state(primitive);
        let message = format!("{error}");
        assert!(message.contains("permission denied"), "display: {message}");
        assert!(message.contains("roles/eng-lead.md"), "display: {message}");
    }

    #[test]
    fn substrate_error_implements_std_error() {
        let primitive = primitive::PrimitiveError::PathPermissionDenied {
            context: primitive::PrimitiveError::context("path permission denied"),
            asset_path: "roles/eng-lead.md".to_string(),
            operation: "get".to_string(),
        };
        let error = SubstrateError::corrupt_persistence_state(primitive);
        let _: &dyn std::error::Error = &error;
    }
}
