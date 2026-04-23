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
    /// Field-level validation rules were violated by entity data.
    ValidationFailed {
        fix = Client,
        recoverability = UserAction,
        hint = "correct the field values reported in the validation errors",
    }
    /// An operation was issued at the wrong point in the checkout lifecycle.
    CheckoutLifecycleViolation {
        fix = Client,
        recoverability = UserAction,
    }
    /// Persist was blocked because the workspace has open checkouts.
    WorkspaceNotClean {
        fix = Client,
        recoverability = UserAction,
        hint = "resolve all pending checkouts before persisting",
    }
    /// The referenced entity does not exist.
    NonExistentData {
        fix = Client,
        recoverability = UserAction,
    }
    /// The entity store channel was unavailable or dropped.
    StoreUnavailable {
        fix = Pari,
        recoverability = NotRecoverable,
    }
    /// An internal Pari invariant was violated.
    PariInvariantViolation {
        fix = Pari,
        recoverability = NotRecoverable,
    }
}
