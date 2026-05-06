//! [`Validator`] — workspace's runner host.
//!
//! Holds a `&'static` reference to the process-wide validation rule
//! registry. Per-entity rule schemas are themselves `&'static` (each
//! entity's `validation_schema()` returns a `OnceLock`-backed static),
//! so today the registry is implicit in the per-kind dispatch.
//! `Validator::new()` is effectively free — it carries no state of its
//! own and lets the workspace stamp one without measurable overhead.
//!
//! Validation is reachable through [`XViewer::validate`] /
//! [`XViewer::validate_with`] (and `XEditor::*` via `Deref`); there is
//! no public free-function entry.

/// Workspace's validation orchestration host.
///
/// Today this is a marker; future iterations may carry a
/// `&'static ValidationRuleSet` aggregating per-entity registrations.
/// The shape is reserved here so callers and downstream code can name
/// the type without needing to revisit when that aggregation lands.
#[derive(Clone, Copy, Default)]
pub struct Validator;

impl Validator {
    /// Stamp a fresh validator. Effectively free.
    pub fn new() -> Self {
        Self
    }
}
