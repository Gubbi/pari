use pari_macros::{ErrorCompose, OTelEmit};

use crate::{
    error::{store::StoreError, BatchError},
    substrate::error::SubstrateError,
    validation::error::ValidationErrors,
};

#[derive(thiserror::Error, Debug, ErrorCompose, OTelEmit)]
pub enum CheckoutError {
    #[error("entity already checked out: {entity_ref}")]
    #[compose(fix = Client, recoverability = UserAction)]
    #[otel(error_type = "already_checked_out")]
    AlreadyCheckedOut { entity_ref: String },

    #[error("entity not found: {entity_ref}")]
    #[compose(fix = Client, recoverability = UserAction)]
    #[otel(error_type = "entity_not_found")]
    EntityNotFound { entity_ref: String },

    #[error(transparent)]
    #[compose(delegate)]
    Substrate(#[from] SubstrateError),

    #[error(transparent)]
    #[compose(delegate)]
    StoreUnavailable(#[from] StoreError),
}

#[derive(thiserror::Error, Debug, ErrorCompose, OTelEmit)]
pub enum CommitError {
    #[error("commit validation failed: {error_count} error(s)")]
    #[compose(fix = Client, recoverability = UserAction)]
    #[otel(error_type = "commit_validation_failed")]
    ValidationFailed {
        #[otel(field = "validation.error_count")]
        error_count: usize,
        errors: ValidationErrors,
    },

    #[error(transparent)]
    #[compose(delegate)]
    CrossReferenceCheckFailed(SubstrateError),

    #[error(transparent)]
    #[compose(delegate)]
    StoreUnavailable(#[from] StoreError),
}

#[derive(thiserror::Error, Debug, ErrorCompose, OTelEmit)]
pub enum LoadError {
    #[error("entity not found: {entity_ref}")]
    #[compose(fix = Client, recoverability = UserAction)]
    #[otel(error_type = "load_entity_not_found")]
    NotFound { entity_ref: String },

    #[error(transparent)]
    #[compose(delegate)]
    Substrate(#[from] SubstrateError),

    #[error("load validation failed: {error_count} error(s)")]
    #[compose(fix = Data, recoverability = OperatorAction)]
    #[otel(error_type = "load_validation_failed")]
    ValidationFailed {
        #[otel(field = "validation.error_count")]
        error_count: usize,
        errors: ValidationErrors,
    },

    #[error(transparent)]
    #[compose(delegate)]
    StoreUnavailable(#[from] StoreError),
}

#[derive(thiserror::Error, Debug, ErrorCompose, OTelEmit)]
pub enum UndoError {
    #[error("wrong state for undo operation")]
    #[compose(fix = Pari, recoverability = NotRecoverable)]
    #[otel(error_type = "wrong_state_for_undo")]
    WrongState,

    #[error(transparent)]
    #[compose(delegate)]
    StoreUnavailable(#[from] StoreError),
}

#[derive(thiserror::Error, Debug, ErrorCompose, OTelEmit)]
pub enum PersistError {
    #[error("persist blocked: {checked_out_count} checkout(s) pending")]
    #[compose(fix = Client, recoverability = UserAction)]
    #[otel(error_type = "pending_checkouts")]
    PendingCheckouts { checked_out_count: usize },

    #[error("{0}")]
    #[compose(delegate)]
    SubstrateErrors(BatchError<SubstrateError>),

    #[error(transparent)]
    #[compose(delegate)]
    StoreUnavailable(#[from] StoreError),
}

#[derive(thiserror::Error, Debug, ErrorCompose, OTelEmit)]
pub enum ResolveError {
    #[error("entity not found: {entity_ref}")]
    #[compose(fix = Client, recoverability = UserAction)]
    #[otel(error_type = "resolve_entity_not_found")]
    NotFound { entity_ref: String },

    #[error(transparent)]
    #[compose(delegate)]
    Substrate(#[from] SubstrateError),

    #[error(transparent)]
    #[compose(delegate)]
    StoreUnavailable(#[from] StoreError),
}
