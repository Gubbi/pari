use crate::{
    entity::{Entity, TrackedEntity},
    error::{primitive::PrimitiveError, ActivityError},
    validation::{kind::ValidationKind, lib::schema::ValidatableTracked},
    workspace::{Workspace, XViewer},
};

/// Runs validation rules for a typed entity through a workspace-bound
/// viewer. Returns `Ok(())` when all rules pass,
/// `Err(ActivityError::ValidationFailed)` when field rules fail, and
/// `Err(ActivityError::PariInvariantViolation)` when a requested field
/// is not in the schema.
pub async fn run_validations<T: Entity>(
    viewer: &XViewer<'_, T>,
    fields: &[&str],
    kinds: &[ValidationKind],
) -> Result<(), ActivityError>
where
    T::Tracked: ValidatableTracked<T>,
{
    crate::validation::lib::runner::run_validations::<T>(viewer, fields, kinds)
        .await
        .map_err(wrap_validation_error)
}

/// Type-erased dispatch — the path used by `StoreServer` orchestration
/// sites that have a `&TrackedEntity`. The workspace argument is the
/// per-request workspace constructed by `StoreServer` over its own
/// dispatcher.
pub async fn run_validations_for_entity(
    workspace: &Workspace,
    entity: &TrackedEntity,
    fields: &[&str],
    kinds: &[ValidationKind],
) -> Result<(), ActivityError> {
    entity
        .run_validations(workspace, fields, kinds)
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
