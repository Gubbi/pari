use std::collections::HashMap;

use crate::{
    entity::Entity,
    error::primitive::PrimitiveError,
    validation::{
        kind::ValidationKind,
        lib::schema::{validate_field_selection, ValidatableTracked},
    },
};

/// Pure validation runner. Accumulates field-level rule failures into a
/// `PrimitiveError::FieldValidationError`. Returns
/// `Err(PrimitiveError::InvalidValidationFieldSelection)` if any requested
/// field name is not in the schema.
pub async fn run_validations<T: Entity>(
    entity: &T::Tracked,
    fields: &[&str],
    kinds: &[ValidationKind],
) -> Result<(), PrimitiveError>
where
    T::Tracked: ValidatableTracked<T>,
{
    let schema = T::validation_schema();
    let mut errors: HashMap<String, Vec<PrimitiveError>> = HashMap::new();

    let all_fields = schema.all_field_names();
    let target_fields: Vec<&str> = if fields.is_empty() {
        all_fields
    } else {
        validate_field_selection(schema, fields)?;
        fields.to_vec()
    };

    for field_name in &target_fields {
        let mut field_errors: Vec<PrimitiveError> = Vec::new();

        if kinds.contains(&ValidationKind::Structural) {
            if let Some(rules) = schema.structural.get(field_name) {
                field_errors.extend(entity.run_structural_rules(field_name, rules));
            }
        }

        if kinds.contains(&ValidationKind::Semantic) {
            if let Some(rules) = schema.semantic.get(field_name) {
                for rule in rules {
                    field_errors.extend(rule(entity).await);
                }
            }
        }

        if kinds.contains(&ValidationKind::CrossEntity) {
            if let Some(rules) = schema.cross_entity.get(field_name) {
                for rule in rules {
                    field_errors.extend(rule(entity).await);
                }
            }
        }

        if !field_errors.is_empty() {
            errors.insert(field_name.to_string(), field_errors);
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(PrimitiveError::field_validation_error(
            "validation failed",
            errors,
        ))
    }
}
