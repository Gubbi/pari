use indexmap::IndexMap;

use super::primitives::{camel_case, min_length};
use crate::{
    entity::{entities::workflow::Step, types::WorkflowStateEntry},
    error::primitive::PrimitiveError,
};

/// Each `steps` map key must be camelCase.
pub fn step_keys_camel_case(steps: &IndexMap<String, Step>) -> Vec<PrimitiveError> {
    let mut v = vec![];
    for key in steps.keys() {
        v.extend(camel_case(key));
    }
    v
}

/// State list validation for workflow states:
/// - At least 2 states
/// - All ids are CamelCase
/// - All ids are unique
/// - At least one Done semantic
/// - At least one non-Done state
pub fn states_valid_workflow(value: &[WorkflowStateEntry]) -> Vec<PrimitiveError> {
    let mut v = vec![];
    v.extend(min_length(value, 2));
    for (i, s) in value.iter().enumerate() {
        let sub = format!("[{i}].id");
        let valid = !s.id.is_empty()
            && s.id.starts_with(|c: char| c.is_ascii_uppercase())
            && s.id.chars().all(|c| c.is_ascii_alphanumeric());
        if !valid {
            v.push(PrimitiveError::naming_format_violation(
                format!("'{}' is not CamelCase", s.id),
                Some(sub),
                "camel_case",
            ));
        }
    }
    let mut seen = std::collections::HashSet::new();
    for (i, s) in value.iter().enumerate() {
        if !seen.insert(s.id.clone()) {
            v.push(PrimitiveError::duplicate_entry_violation(
                "duplicate entry",
                format!("[{i}].id"),
                "unique",
            ));
        }
    }
    let has_done = value.iter().any(|s| {
        matches!(
            s.semantic,
            Some(crate::entity::types::WorkflowSemantic::Done)
        )
    });
    let has_non_done = value.iter().any(|s| {
        !matches!(
            s.semantic,
            Some(crate::entity::types::WorkflowSemantic::Done)
        )
    });
    if !has_done {
        v.push(PrimitiveError::workflow_graph_inconsistency(
            "must include at least one Done state",
            "missing_done_semantic",
        ));
    }
    if !has_non_done {
        v.push(PrimitiveError::workflow_graph_inconsistency(
            "must include at least one non-Done state",
            "all_done_states",
        ));
    }
    v
}
