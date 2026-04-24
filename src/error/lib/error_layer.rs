//! Tier tag on every Pari error.
//!
//! Pari's errors form a three-tier chain: Job → Activity → Primitive. Every
//! error knows which tier it is in — callers use this to reason about the
//! chain without downcasting or string-matching on the concrete type.
//!
//! See [`docs/design/layers/error-handling.md`](../../../../docs/design/layers/error-handling.md)
//! for the tier model.

/// Which tier a concrete error sits at in the Job → Activity → Primitive chain.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorLayer {
    /// Atomic leaf failure. Carries diagnostics (`message`, location, traces).
    Primitive,
    /// Subsystem outcome. Declares classification; carries `component` + `hint`.
    Activity,
    /// Client-intent framing of a completed operation outcome.
    Job,
}
