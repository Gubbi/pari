//! [`Validator`] — workspace's runner host.
//!
//! Holds a `&'static ValidationRuleSet` reference. The set is built
//! once at first use via a [`LazyLock`] and reused across every
//! validator stamped through [`Validator::new`].
//!
//! `Validator::new()` is effectively free — it stamps the static
//! reference and carries no state of its own. Per-entity rule schemas
//! are themselves `&'static` (each entity's `validation_schema()`
//! returns a `OnceLock`-backed static), so the rule set is a
//! forward-looking aggregation seam: today it carries no per-entity
//! entries, but its shape is reserved here so future runtime rule
//! registration can land without restructuring the runner host.
//!
//! Validation is reachable through [`XViewer::validate`] /
//! [`XViewer::validate_with`] (and `XEditor::*` via `Deref`); each of
//! those routes through `Workspace::validator().run::<T>(...)`. There
//! is no public free-function entry into the runner.

use std::sync::LazyLock;

use crate::{
    entity::Entity,
    error::ActivityError,
    validation::{lib::schema::ValidatableTracked, ValidationKind},
    workspace::viewer::XViewer,
};

/// Aggregated validation rule set. Built once, shared by every
/// [`Validator`] in the process.
///
/// Today this is a placeholder — per-entity rule schemas live behind
/// each entity's `validation_schema()` static and are reached via
/// generic dispatch on `T: Entity`. The struct is reserved here so
/// future runtime rule registration (additions, overrides) can land
/// alongside the static-schema path without reshaping the validator
/// API.
pub struct ValidationRuleSet {
    _private: (),
}

impl ValidationRuleSet {
    fn new() -> Self {
        Self { _private: () }
    }
}

static REGISTRY: LazyLock<ValidationRuleSet> = LazyLock::new(ValidationRuleSet::new);

/// Workspace's validation orchestration host.
#[derive(Clone, Copy)]
pub struct Validator {
    #[allow(dead_code)]
    rules: &'static ValidationRuleSet,
}

impl Validator {
    /// Stamp a fresh validator. Effectively free — one static
    /// reference clone.
    pub fn new() -> Self {
        Self { rules: &REGISTRY }
    }

    /// Run the validation runner against a workspace-bound viewer.
    /// The single orchestration entry the rest of the crate calls;
    /// `XViewer::validate` and `XViewer::validate_with` route through
    /// here.
    pub(crate) async fn run<T: Entity>(
        &self,
        viewer: &XViewer<'_, T>,
        fields: &[&str],
        kinds: &[ValidationKind],
    ) -> Result<(), ActivityError>
    where
        <T as Entity>::Tracked: ValidatableTracked<T>,
    {
        crate::validation::run_validations::<T>(viewer, fields, kinds).await
    }
}

impl Default for Validator {
    fn default() -> Self {
        Self::new()
    }
}
