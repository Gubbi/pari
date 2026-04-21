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
        assert!(
            message.contains("corrupt persistence state"),
            "display: {message}"
        );
        assert!(message.contains("permission denied"), "display: {message}");
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
