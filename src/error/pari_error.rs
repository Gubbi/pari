//! `PariError` — the Job tier of Pari's error chain.
//!
//! `PariError` is the single umbrella error type every integrator imports.
//! Everything Pari can fail at surfaces through a variant of this enum, which
//! lets callers write one match over one type rather than stringing together
//! per-layer error types.
//!
//! # Shape
//!
//! Each variant names an **operation outcome in client-intent terms**
//! (`DefinitionRejected`, `SaveFailed`, `CheckoutFailed`, …) and wraps an
//! `ActivityError` that carries the subsystem-level framing, classification,
//! and the primitive cause. Variants delegate classification via
//! `#[compose(delegate)]` — the Activity tier is the single source of truth.
//!
//! The Job tier is deliberately framed in **product / business language**,
//! not in terms of which internal component ran. A single variant here may be
//! reached through several different activity/primitive combinations; the
//! classification and diagnostics travel with those lower tiers and are
//! reachable through the `source()` chain or `as_error::<E>()`.
//!
//! # Why not one enum per Job
//!
//! Early designs imagined a distinct Job enum per caller operation. In
//! practice, integrators want one import, one match, one mapping into their
//! own error type — not a forest of Job enums that each have to be composed
//! separately. A single `PariError` gives that ergonomics without losing
//! expressivity: the per-operation framing lives in the variant name, and
//! everything below it is reachable through the chain.
//!
//! # Usage
//!
//! Callers typically match on `err.recoverability()` to decide their action,
//! call `err.emit()` to push a structured event to OTel, and optionally reach
//! for a specific concrete type via `err.as_error::<SomePrimitive>()`.

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
