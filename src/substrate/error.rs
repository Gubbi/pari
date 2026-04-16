//! `SubstrateError` — substrate boundary error enum.

use pari_macros::{ErrorCompose, OTelEmit};

use crate::substrate::pipeline::{codec::CodecError, executor::ExecutorError};

#[derive(thiserror::Error, Debug, ErrorCompose, OTelEmit)]
pub enum SubstrateError {
    #[error(transparent)]
    #[compose(delegate)]
    Codec(#[from] CodecError),

    #[error(transparent)]
    #[compose(delegate)]
    Executor(#[from] ExecutorError),
}
