//! Validation rule kind classification.

/// Selects which validation rule kinds run during a validation pass.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidationKind {
    Structural,
    Semantic,
    CrossEntity,
}
