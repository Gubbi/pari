//! `ActivityError` — centralized cross-layer activity error enum.
//!
//! Each variant represents an error at an orchestration boundary.
//! Pure `lib/` components emit `PrimitiveError`; orchestration
//! components at the layer root wrap them into `ActivityError`.

use pari_macros::activity_errors;

use crate::error::{
    lib::{ErrorCompose, ErrorLayer, FixDomain, OTelEmit, Recoverability},
    primitive::PrimitiveError,
};

activity_errors! {
    /// Schema or pipeline field-mapping error.
    InvalidPersistenceLayout {
        fix = Data,
        recoverability = OperatorAction,
        hint = "check the entity schema definition and field mappings",
    }
    /// Entity could not be serialized or deserialized.
    UnpersistableDefinition {
        fix = Data,
        recoverability = OperatorAction,
    }
    /// Persistence state is corrupt or inconsistent.
    CorruptPersistenceState {
        fix = Infra,
        recoverability = OperatorAction,
    }
}
