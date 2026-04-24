//! `Recoverability` — what the caller should do about an error.
//!
//! One of Pari's two classification axes (the other is [`FixDomain`]). Callers
//! match on `err.recoverability()` to decide *retry / surface to user / alert
//! operator / escalate as a Pari bug* without string-matching.
//!
//! Declared once per Activity variant and propagated through the chain by
//! `ErrorCompose`.
//!
//! [`FixDomain`]: super::FixDomain

/// What the caller should do in response to a failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Recoverability {
    /// Transient failure — retry automatically after backoff.
    Retryable,
    /// Caller must fix their input or definition, then retry.
    UserAction,
    /// Operator must fix infrastructure or data, then retry.
    OperatorAction,
    /// Code invariant violated — do not retry, escalate to developer.
    NotRecoverable,
}
