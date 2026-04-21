//! Validation data types and `SetterError`.

use pari_macros::{ErrorCompose, OTelEmit};

use crate::error::{primitive::PrimitiveError, ActivityError};

// ---------------------------------------------------------------------------
// Plain data types (not ErrorCompose)
// ---------------------------------------------------------------------------

#[derive(Debug)]
pub struct ValidationErrors {
    pub errors: Vec<FieldValidationError>,
}

impl ValidationErrors {
    pub fn new() -> Self {
        Self { errors: vec![] }
    }
    pub fn is_empty(&self) -> bool {
        self.errors.is_empty()
    }
    pub fn extend(&mut self, other: ValidationErrors) {
        self.errors.extend(other.errors);
    }
}

impl Default for ValidationErrors {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidationKind {
    Structural,
    Semantic,
    CrossEntity,
}

#[derive(Debug)]
pub struct FieldValidationError {
    /// Schema field name: `"id"`, `"steps"`, `"name"`
    pub path: String,
    pub error: PrimitiveError,
}

// ---------------------------------------------------------------------------
// SetterError
// ---------------------------------------------------------------------------

#[derive(thiserror::Error, Debug, ErrorCompose, OTelEmit)]
pub enum SetterError {
    /// ensure_mutable triggered a substrate load which failed.
    #[error(transparent)]
    #[compose(delegate)]
    Substrate(#[from] ActivityError),

    /// Structural or semantic validation rejected the incoming value.
    #[error("validation failed: {error_count} error(s)")]
    #[compose(fix = Client, recoverability = UserAction)]
    #[otel(error_type = "setter_validation_failed")]
    Validation {
        #[otel(field = "validation.error_count")]
        error_count: usize,
        errors: ValidationErrors,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stub_error() -> PrimitiveError {
        PrimitiveError::empty_required_value("test", None::<String>, "non_empty")
    }

    #[test]
    fn validation_errors_starts_empty() {
        let e = ValidationErrors::new();
        assert!(e.is_empty());
    }

    #[test]
    fn validation_errors_extend_combines_errors() {
        let mut e1 = ValidationErrors::new();
        e1.errors.push(FieldValidationError {
            path: "name".to_string(),
            error: stub_error(),
        });
        let e2 = ValidationErrors::new();
        e1.extend(e2);
        assert_eq!(e1.errors.len(), 1);
    }
}
