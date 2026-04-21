#[cfg(test)]
mod tests {
    use crate::{
        error::{primitive, ActivityError},
        substrate::{RepoSubstrate, Substrate},
    };

    #[test]
    fn activity_error_display_format() {
        let primitive = primitive::PrimitiveError::PathPermissionDenied {
            context: primitive::PrimitiveError::context("path permission denied"),
            asset_path: "roles/eng-lead.md".to_string(),
            operation: "get".to_string(),
        };
        let error =
            ActivityError::corrupt_persistence_state(RepoSubstrate::substrate_name(), primitive);
        let message = format!("{error}");
        assert!(message.contains("permission denied"), "display: {message}");
    }

    #[test]
    fn activity_error_implements_std_error() {
        let primitive = primitive::PrimitiveError::PathPermissionDenied {
            context: primitive::PrimitiveError::context("path permission denied"),
            asset_path: "roles/eng-lead.md".to_string(),
            operation: "get".to_string(),
        };
        let error =
            ActivityError::corrupt_persistence_state(RepoSubstrate::substrate_name(), primitive);
        let _: &dyn std::error::Error = &error;
    }
}
