//! `ActivityError` — the Activity tier of Pari's error chain.
//!
//! The Activity tier is where a Pari error acquires **meaning for an
//! integrator**. A primitive says "a file write failed at this line"; an
//! activity says "persistence is corrupt — operator must fix infrastructure
//! before retry". That re-framing is why this tier exists as a distinct layer
//! in the chain rather than a thin wrapper around primitives.
//!
//! # Contract
//!
//! Every activity variant carries a fixed shape:
//!
//! - **Classification** — `fix: FixDomain` + `recoverability: Recoverability`,
//!   declared as macro arguments per variant. These are the properties callers
//!   read via `err.fix_domain()` / `err.recoverability()`. Severity is derived.
//! - **`component`** — an auto-injected identifier naming the subsystem that
//!   surfaced the failure. Fixed per variant, not per instance. Exposed to OTel
//!   as the shared `error.component` field so integrators can filter or route
//!   on it without touching the variant name.
//! - **`hint`** — a `&'static str` of corrective guidance, fixed per variant
//!   and required at the macro call site. Every Activity variant ships a hint
//!   so integrators always receive remediation direction. Exposed to OTel as
//!   `error.hint`.
//! - **`cause: PrimitiveError`** — the concrete leaf that triggered this
//!   activity outcome. Auto-added. Reachable via `std::error::Error::source()`
//!   and carries the diagnostics (message, location, span trace, backtrace).
//!
//! The centralized enum shape — one `ActivityError` enum across layers rather
//! than one per owning module — is a deliberate choice: activity outcomes are
//! framed in product / business language and are independent of which Pari
//! component happened to run. The `component` field supplies the code-side
//! identity; the variant name supplies the product-side identity.
//!
//! # Why a declarative macro
//!
//! `activity_errors! { ... }` exists to make the cost of adding a new variant
//! small and the shape of variants uniform. A new activity outcome requires
//! one block with classification and a hint; every boilerplate
//! concern — `thiserror`, `ErrorCompose`, `OTelEmit`, `component`, `cause`,
//! the accessors callers read — is generated.
//!
//! # Usage
//!
//! Orchestration code produces an `ActivityError` by mapping a `PrimitiveError`
//! coming out of a pure `lib/` component into the right activity variant.
//! Callers at the Job tier then wrap the `ActivityError` into a `PariError`
//! variant. Typed inspection of the primitive is available via
//! `err.as_error::<PrimitiveError>()` chained from an activity-tier handle —
//! see [`ErrorCompose::as_error`](../lib/compose.rs).
//!
//! Generation mechanics live in
//! [`pari-macros::activity_error_enum`](../../../pari-macros/src/activity_error_enum.rs).

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
        hint = "check the entity's Serialize/Deserialize derive and schema for type mismatches",
    }
    /// Persistence state is corrupt or inconsistent.
    CorruptPersistenceState {
        fix = Infra,
        recoverability = OperatorAction,
        hint = "inspect the persistence layout for partial writes or external edits and restore from a known-good state",
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
        hint = "ensure the entity is checked out before mutating and committed before dropping the handle",
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
        hint = "verify the entity id and parent ref, and confirm the entity has been created and persisted",
    }
    /// The entity store channel was unavailable or dropped.
    StoreUnavailable {
        fix = Pari,
        recoverability = NotRecoverable,
        hint = "the entity store task has stopped; restart the workspace to recover",
    }
    /// An internal Pari invariant was violated.
    PariInvariantViolation {
        fix = Pari,
        recoverability = NotRecoverable,
        hint = "file a bug with the emitted span trace and backtrace — this indicates a defect in Pari itself",
    }
}
