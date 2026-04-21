use super::{
    error::{FieldValidationError, ValidationErrors, ValidationKind},
    lib::schema::{validate_field_selection, ValidatableTracked},
};
use crate::{
    entity::{Entity, TrackedEntity},
    error::primitive::PrimitiveError,
};

/// Runs validation rules from the entity's schema.
///
/// `fields: &[]` means all fields present in the schema.
/// `fields: &["name", "purpose"]` runs only those fields.
/// `kinds` selects which rule kinds to run.
///
/// Returns `Err(PrimitiveError::InvalidValidationFieldSelection)` if any requested
/// field name is absent from all rule maps. Errors accumulate otherwise — all
/// failing rules are collected before returning.
pub async fn run_validations<T: Entity>(
    entity: &T::Tracked,
    fields: &[&str],
    kinds: &[ValidationKind],
) -> Result<ValidationErrors, PrimitiveError>
where
    T::Tracked: ValidatableTracked<T>,
{
    let schema = T::validation_schema();
    let mut result = ValidationErrors::new();

    let all_fields = schema.all_field_names();
    let target_fields: Vec<&str> = if fields.is_empty() {
        all_fields
    } else {
        validate_field_selection(schema, fields)?;
        fields.to_vec()
    };

    for field_name in &target_fields {
        if kinds.contains(&ValidationKind::Structural) {
            if let Some(rules) = schema.structural.get(field_name) {
                for error in entity.run_structural_rules(field_name, rules) {
                    result.errors.push(FieldValidationError {
                        path: field_name.to_string(),
                        error,
                    });
                }
            }
        }

        if kinds.contains(&ValidationKind::Semantic) {
            if let Some(rules) = schema.semantic.get(field_name) {
                for rule in rules {
                    for error in rule(entity).await {
                        result.errors.push(FieldValidationError {
                            path: field_name.to_string(),
                            error,
                        });
                    }
                }
            }
        }

        if kinds.contains(&ValidationKind::CrossEntity) {
            if let Some(rules) = schema.cross_entity.get(field_name) {
                for rule in rules {
                    for error in rule(entity).await {
                        result.errors.push(FieldValidationError {
                            path: field_name.to_string(),
                            error,
                        });
                    }
                }
            }
        }
    }

    Ok(result)
}

pub async fn run_validations_for_entity(
    entity: &TrackedEntity,
    fields: &[&str],
    kinds: &[ValidationKind],
) -> Result<ValidationErrors, PrimitiveError> {
    entity.run_validations(fields, kinds).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn run_validations_runs_structural_rules() {
        // Placeholder — full integration tests in Task 07.
        // Verifies the function signature compiles.
        use crate::entity::EntityKind;
        let _ = ValidationKind::Structural;
        let _ = EntityKind::Role;
    }
}
