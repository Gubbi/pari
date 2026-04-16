//! Validation data types and `SetterError`.

use pari_macros::{ErrorCompose, OTelEmit};

use crate::substrate::error::SubstrateError;

// ---------------------------------------------------------------------------
// Plain data types (not ErrorCompose)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
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

#[derive(Debug, Clone)]
pub struct FieldValidationError {
    /// Dot-notation path: `"id"`, `"steps.WriteProposal.depends_on"`
    pub path: String,
    pub message: String,
    pub kind: ValidationKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidationKind {
    Structural,
    Semantic,
    CrossEntity,
}

// ---------------------------------------------------------------------------
// SetterError
// ---------------------------------------------------------------------------

#[derive(thiserror::Error, Debug, ErrorCompose, OTelEmit)]
pub enum SetterError {
    /// ensure_mutable triggered a substrate load which failed.
    #[error(transparent)]
    #[compose(delegate)]
    Substrate(#[from] SubstrateError),

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
