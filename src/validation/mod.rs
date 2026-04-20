//! Validation framework.
//!
//! [`ValidationSchema`] — per-entity schema with three rule maps.
//! [`run_validations`] — async runner that dispatches rules and accumulates errors.
//! Shared structural primitive rule functions.

pub mod artifact_kind;
pub mod cross_entity;
pub mod error;
pub mod hook;
pub mod primitives;
pub mod relay;
pub mod role;
pub mod runner;
pub mod schema;
pub mod task;
pub mod team;
pub mod workflow;

pub use error::{FieldValidationError, SetterError, ValidationErrors, ValidationKind};
pub use primitives::{
    camel_case, camel_case_id, kebab_case, kebab_case_id, min_length, non_empty_list,
    non_empty_str, raci_structural, states_valid_task, states_valid_workflow, unique_by,
    x_prefix_keys,
};
pub use runner::{run_validations, run_validations_for_entity};
pub use schema::{
    AnyCrossEntityRule, AnySemanticRule, AnyStructuralRule, ValidatableTracked, ValidationSchema,
};
