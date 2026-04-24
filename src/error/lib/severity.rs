//! `Severity` — derived log-level classification.
//!
//! Severity is a function of [`FixDomain`] and [`Recoverability`], not an
//! independent axis. Deriving it rather than declaring it prevents the common
//! drift where an error says `Error` but classifies as a recoverable user
//! input problem (or vice versa).
//!
//! Callers rarely branch on `Severity` directly — `Recoverability` is the
//! action-oriented axis. `Severity` is for log routing and alert thresholds.

use super::{FixDomain, Recoverability};

/// Log-level classification for an error. Always derived from classification,
/// never declared by the author.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    /// Expected or low-impact condition — typically transient infra failures
    /// or correctable client input.
    Warn,
    /// Unexpected or high-impact condition — corrupt data, operator-action
    /// infra failures, or Pari invariant violations.
    Error,
}

impl Severity {
    /// Compute severity from the two classification axes.
    ///
    /// Every known pair maps deterministically; see
    /// [`docs/design/layers/error-handling.md`](../../../../docs/design/layers/error-handling.md#severity-derived-never-declared)
    /// for the full table. Any combination not explicitly listed falls through
    /// to `Error` — there is no "unknown" level.
    pub fn from_classification(fix: FixDomain, recoverability: Recoverability) -> Self {
        match (fix, recoverability) {
            (FixDomain::Pari, Recoverability::NotRecoverable) => Severity::Error,
            (FixDomain::Data, Recoverability::OperatorAction) => Severity::Error,
            (FixDomain::Infra, Recoverability::OperatorAction) => Severity::Error,
            (FixDomain::Infra, Recoverability::Retryable) => Severity::Warn,
            (FixDomain::Client, Recoverability::UserAction) => Severity::Warn,
            _ => Severity::Error,
        }
    }
}
