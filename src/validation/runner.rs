use crate::{
    entity::{Entity, TrackedEntity},
    error::{primitive::PrimitiveError, ActivityError},
    validation::{kind::ValidationKind, lib::schema::ValidatableTracked},
};

/// Runs validation rules for a typed entity. Returns `Ok(())` when all rules
/// pass, `Err(ActivityError::ValidationFailed)` when field rules fail, and
/// `Err(ActivityError::PariInvariantViolation)` when a requested field is not
/// in the schema.
pub async fn run_validations<T: Entity>(
    entity: &T::Tracked,
    fields: &[&str],
    kinds: &[ValidationKind],
) -> Result<(), ActivityError>
where
    T::Tracked: ValidatableTracked<T>,
{
    crate::validation::lib::runner::run_validations::<T>(entity, fields, kinds)
        .await
        .map_err(wrap_validation_error)
}

/// Dispatches through the type-erased `TrackedEntity` wrapper.
pub async fn run_validations_for_entity(
    entity: &TrackedEntity,
    fields: &[&str],
    kinds: &[ValidationKind],
) -> Result<(), ActivityError> {
    entity
        .run_validations(fields, kinds)
        .await
        .map_err(wrap_validation_error)
}

fn wrap_validation_error(p: PrimitiveError) -> ActivityError {
    if matches!(p, PrimitiveError::InvalidValidationFieldSelection { .. }) {
        ActivityError::pari_invariant_violation("validation.runner", p)
    } else {
        ActivityError::validation_failed("validation.runner", p)
    }
}
