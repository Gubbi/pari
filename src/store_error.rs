//! Store-layer channel/actor boundary errors.

use pari_macros::{ErrorCompose, OTelEmit};

#[derive(thiserror::Error, Debug, ErrorCompose, OTelEmit)]
pub enum StoreError {
    #[error("entity server unavailable")]
    #[compose(fix = Pari, recoverability = NotRecoverable)]
    #[otel(error_type = "store_unavailable")]
    Unavailable,
}
