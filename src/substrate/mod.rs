//! Substrate layer — persistence backend trait and implementations.

pub mod repo;

pub use crate::schema::store::EntityStore;

/// A filesystem path + human-readable description of what went wrong.
#[derive(Debug, thiserror::Error)]
#[error("{message} at {path}")]
pub struct SubstrateError {
    pub path: String,
    pub message: String,
}

/// Persistence backend interface. All implementations must support `persist`.
/// `load` will be added in a subsequent proposal.
pub trait Substrate {
    /// Write all entities in `store` to the backend.
    ///
    /// # Errors
    ///
    /// Returns a non-empty `Vec<SubstrateError>` if any entity could not be written.
    /// All entities are attempted; errors are collected rather than short-circuited.
    fn persist(&self, store: &EntityStore) -> Result<(), Vec<SubstrateError>>;
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- 6.1: SubstrateError Display and std::error::Error tests ---

    #[test]
    fn substrate_error_display_format() {
        let e = SubstrateError {
            path: "roles/eng-lead.md".to_string(),
            message: "permission denied".to_string(),
        };
        assert_eq!(format!("{}", e), "permission denied at roles/eng-lead.md");
    }

    #[test]
    fn substrate_error_implements_std_error() {
        let e = SubstrateError {
            path: "roles/eng-lead.md".to_string(),
            message: "permission denied".to_string(),
        };
        let _: &dyn std::error::Error = &e;
    }

    // --- 11.1: Substrate trait contract tests ---

    #[test]
    fn substrate_error_has_path_and_message() {
        let e = SubstrateError {
            path: "roles/eng-lead.md".to_string(),
            message: "permission denied".to_string(),
        };
        assert_eq!(e.path, "roles/eng-lead.md");
        assert_eq!(e.message, "permission denied");
    }

    #[test]
    fn entity_store_holds_entity_collections() {
        let store = EntityStore::new();
        assert!(store.roles.is_empty());
        assert!(store.hooks.is_empty());
        assert!(store.teams.is_empty());
        assert!(store.workflows.is_empty());
        assert!(store.shared_workflows.is_empty());
    }
}
