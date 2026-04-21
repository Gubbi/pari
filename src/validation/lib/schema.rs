use std::collections::HashMap;

use crate::{entity::Entity, error::primitive::PrimitiveError};

// ---------------------------------------------------------------------------
// Rule function type aliases
// ---------------------------------------------------------------------------

/// Structural rule: sync closure that receives the whole tracked entity.
pub type AnyStructuralRule<E> =
    Box<dyn Fn(&<E as Entity>::Tracked) -> Vec<PrimitiveError> + Send + Sync>;

/// Semantic rule: async closure that receives the whole tracked entity.
pub type AnySemanticRule<E> = Box<
    dyn for<'a> Fn(
            &'a <E as Entity>::Tracked,
        ) -> std::pin::Pin<
            Box<dyn std::future::Future<Output = Vec<PrimitiveError>> + Send + 'a>,
        > + Send
        + Sync,
>;

/// Cross-entity rule: same signature as a semantic rule.
pub type AnyCrossEntityRule<E> = AnySemanticRule<E>;

// ---------------------------------------------------------------------------
// ValidationSchema
// ---------------------------------------------------------------------------

/// Per-entity validation schema.
/// Three maps from field name → list of rules.
/// A field absent from a map has no rules of that kind.
pub struct ValidationSchema<E: Entity> {
    pub structural: HashMap<&'static str, Vec<AnyStructuralRule<E>>>,
    pub semantic: HashMap<&'static str, Vec<AnySemanticRule<E>>>,
    pub cross_entity: HashMap<&'static str, Vec<AnyCrossEntityRule<E>>>,
}

impl<E: Entity> ValidationSchema<E> {
    pub fn empty() -> Self {
        Self {
            structural: HashMap::new(),
            semantic: HashMap::new(),
            cross_entity: HashMap::new(),
        }
    }

    /// All field names that appear in any rule map.
    pub fn all_field_names(&self) -> Vec<&str> {
        let mut fields: std::collections::HashSet<&str> = std::collections::HashSet::new();
        for k in self.structural.keys() {
            fields.insert(k);
        }
        for k in self.semantic.keys() {
            fields.insert(k);
        }
        for k in self.cross_entity.keys() {
            fields.insert(k);
        }
        fields.into_iter().collect()
    }
}

// ---------------------------------------------------------------------------
// ValidatableTracked
// ---------------------------------------------------------------------------

/// Implemented by every `TrackedX` struct (blanket impl below).
/// Provides runtime dispatch from field name → structural rule execution.
pub trait ValidatableTracked<E: Entity> {
    fn run_structural_rules(
        &self,
        field_name: &str,
        rules: &[AnyStructuralRule<E>],
    ) -> Vec<PrimitiveError>;
}

impl<E: Entity> ValidatableTracked<E> for E::Tracked {
    fn run_structural_rules(
        &self,
        _field_name: &str,
        rules: &[AnyStructuralRule<E>],
    ) -> Vec<PrimitiveError> {
        rules.iter().flat_map(|r| r(self)).collect()
    }
}

// ---------------------------------------------------------------------------
// Pure field selection check
// ---------------------------------------------------------------------------

/// Validates that every requested field name exists in at least one rule map.
/// Returns `Err(PrimitiveError::InvalidValidationFieldSelection)` on the first
/// unknown field; returns `Ok(())` if all fields are known or the slice is empty.
pub fn validate_field_selection<E: Entity>(
    schema: &ValidationSchema<E>,
    fields: &[&str],
) -> Result<(), PrimitiveError> {
    for field_name in fields {
        let in_any_map = schema.structural.contains_key(field_name)
            || schema.semantic.contains_key(field_name)
            || schema.cross_entity.contains_key(field_name);
        if !in_any_map {
            return Err(PrimitiveError::invalid_validation_field_selection(
                format!("field '{field_name}' is not in the validation schema"),
                *field_name,
            ));
        }
    }
    Ok(())
}
