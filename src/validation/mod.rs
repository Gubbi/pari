//! Validation framework.
//!
//! [`ValidationSchema`] — per-entity schema with three rule maps.
//! [`run_validations`] — async runner that dispatches rules and accumulates errors.

pub mod error;
mod lib;
mod runner;

pub use error::{FieldValidationError, SetterError, ValidationErrors, ValidationKind};
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
