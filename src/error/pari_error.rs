//! `PariError` — job-layer error enum.

use pari_macros::{ErrorCompose, OTelEmit};

use crate::error::ActivityError;

#[derive(thiserror::Error, Debug, ErrorCompose, OTelEmit)]
pub enum PariError {
    #[error(transparent)]
    #[compose(delegate)]
    DefinitionRejected(ActivityError),

    #[error(transparent)]
    #[compose(delegate)]
    MutationFailed(ActivityError),

    #[error(transparent)]
    #[compose(delegate)]
    CheckoutFailed(ActivityError),

    #[error(transparent)]
    #[compose(delegate)]
    LoadFailed(ActivityError),

    #[error(transparent)]
    #[compose(delegate)]
    ResolveFailed(ActivityError),

    #[error(transparent)]
    #[compose(delegate)]
    SaveFailed(ActivityError),

    #[error(transparent)]
    #[compose(delegate)]
    SetterRejected(ActivityError),
}
