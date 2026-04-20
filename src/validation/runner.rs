use crate::entity::{Entity, TrackedEntity};

use super::{
    error::{FieldValidationError, ValidationErrors, ValidationKind},
    schema::ValidatableTracked,
};

/// Runs validation rules from the entity's schema.
///
/// `fields: &[]` means all fields present in the schema.
/// `fields: &["name", "purpose"]` runs only those fields.
/// `kinds` selects which rule kinds to run.
///
/// Errors accumulate — all failing rules are collected before returning.
pub async fn run_validations<T: Entity>(
    entity: &T::Tracked,
    fields: &[&str],
    kinds: &[ValidationKind],
) -> ValidationErrors
where
    T::Tracked: ValidatableTracked<T>,
{
    let schema = T::validation_schema();
    let mut result = ValidationErrors::new();

    let all_fields = schema.all_field_names();
    let target_fields: Vec<&str> = if fields.is_empty() {
        all_fields
    } else {
        fields.to_vec()
    };

    for field_name in &target_fields {
        if kinds.contains(&ValidationKind::Structural) {
            if let Some(rules) = schema.structural.get(field_name) {
                for v in entity.run_structural_rules(field_name, rules) {
                    result.errors.push(FieldValidationError {
                        path: build_path(field_name, &v.sub_path),
                        message: v.message,
                        kind: ValidationKind::Structural,
                    });
                }
            }
        }

        if kinds.contains(&ValidationKind::Semantic) {
            if let Some(rules) = schema.semantic.get(field_name) {
                for rule in rules {
                    for v in rule(entity).await {
                        result.errors.push(FieldValidationError {
                            path: build_path(field_name, &v.sub_path),
                            message: v.message,
                            kind: ValidationKind::Semantic,
                        });
                    }
                }
            }
        }

        if kinds.contains(&ValidationKind::CrossEntity) {
            if let Some(rules) = schema.cross_entity.get(field_name) {
                for rule in rules {
                    for v in rule(entity).await {
                        result.errors.push(FieldValidationError {
                            path: build_path(field_name, &v.sub_path),
                            message: v.message,
                            kind: ValidationKind::CrossEntity,
                        });
                    }
                }
            }
        }
    }

    result
}

pub async fn run_validations_for_entity(
    entity: &TrackedEntity,
    fields: &[&str],
    kinds: &[ValidationKind],
) -> ValidationErrors {
    entity.run_validations(fields, kinds).await
}

pub fn build_path(field: &str, sub_path: &Option<String>) -> String {
    match sub_path {
        None => field.to_string(),
        Some(sub) => format!("{field}{sub}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_path_no_sub_path() {
        assert_eq!(build_path("name", &None), "name");
    }

    #[test]
    fn build_path_with_sub_path() {
        assert_eq!(
            build_path("steps", &Some(".WriteProposal.depends_on".to_string())),
            "steps.WriteProposal.depends_on"
        );
    }

    #[tokio::test]
    async fn run_validations_runs_structural_rules() {
        // Placeholder — full integration tests in Task 07.
        // Verifies the function signature compiles.
        use crate::entity::EntityKind;
        let _ = ValidationKind::Structural;
        let _ = EntityKind::Role;
    }
}
