//! Validation layer — owns *what* is valid, not *when* validation runs.
//!
//! Each entity declares a static [`ValidationSchema`] of field-level
//! rules split across three kinds: structural (sync, value-only),
//! semantic (async, entity-local), and cross-entity (async, queries
//! the store via `EntityClient::has_ref`). The runner
//! ([`run_validations`] for typed callers, [`run_validations_for_entity`]
//! for the type-erased `TrackedEntity` wrapper) executes the selected
//! `(fields × kinds)` combination and accumulates failures.
//!
//! Callers choose the `(fields, kinds)` tuple that matches their
//! context: generated setters run Structural+Semantic on the one
//! field they touch; `EntityServer` runs the full gate at insert,
//! load, and commit. See `docs/design/layers/validation.md` for the
//! full decision table.

mod kind;
pub mod lib;
mod runner;

pub use kind::ValidationKind;
// Re-export entity rule modules so paths like `crate::validation::workflow::workflow_validation_schema`
// remain valid for entity schema attributes.
pub use lib::rules::{artifact_kind, hook, relay, role, task, team, workflow};
pub use lib::{
    rules::structural::{
        primitives::{
            camel_case, camel_case_id, kebab_case, kebab_case_id, min_length, non_empty_list,
            non_empty_str, opt_non_empty_str, unique_by, x_prefix_keys,
        },
        raci::raci_structural,
        task::states_valid_task,
        workflow::states_valid_workflow,
    },
    schema::{
        AnyCrossEntityRule, AnySemanticRule, AnyStructuralRule, ValidatableTracked,
        ValidationSchema,
    },
};
pub use runner::{run_validations, run_validations_for_entity};
