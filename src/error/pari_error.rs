//! `PariError` — job-layer error enum.

use pari_macros::{ErrorCompose, OTelEmit};

use crate::{
    store::{CheckoutError, CommitError, LoadError, PersistError, ResolveError},
    validation::error::SetterError,
};

#[derive(thiserror::Error, Debug, ErrorCompose, OTelEmit)]
pub enum PariError {
    #[error(transparent)]
    #[compose(delegate)]
    DefinitionRejected(#[from] CommitError),

    #[error(transparent)]
    #[compose(delegate)]
    MutationFailed(CommitError),

    #[error(transparent)]
    #[compose(delegate)]
    CheckoutFailed(#[from] CheckoutError),

    #[error(transparent)]
    #[compose(delegate)]
    LoadFailed(#[from] LoadError),

    #[error(transparent)]
    #[compose(delegate)]
    ResolveFailed(#[from] ResolveError),

    #[error(transparent)]
    #[compose(delegate)]
    SaveFailed(#[from] PersistError),

    #[error(transparent)]
    #[compose(delegate)]
    SetterRejected(#[from] SetterError),
}
