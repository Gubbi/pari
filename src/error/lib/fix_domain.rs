//! `FixDomain` — which domain owns the fix for an error.
//!
//! One of Pari's two classification axes (the other is [`Recoverability`]).
//! `FixDomain` is descriptive, not accusatory — it answers *where does someone
//! need to act to resolve this?*, not *who is to blame?*.
//!
//! Declared once per Activity variant and propagated through the chain by
//! `ErrorCompose`. Callers read it via `err.fix_domain()`.
//!
//! [`Recoverability`]: super::Recoverability

/// Which domain owns the resolution for a failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FixDomain {
    /// Fix is in the caller's input or usage.
    Client,
    /// Fix requires repairing stored content (corrupt or malformed).
    Data,
    /// Fix is in the underlying infrastructure (permissions, disk, network).
    Infra,
    /// Fix is in Pari's code (invariant violated, logic bug).
    Pari,
}
