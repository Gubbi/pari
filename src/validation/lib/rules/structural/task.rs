use super::primitives::min_length;
use crate::{entity::types::TaskStateEntry, error::primitive::PrimitiveError};

/// State list validation for task states — same rules as workflow but for `TaskSemantic`.
pub fn states_valid_task(value: &[TaskStateEntry]) -> Vec<PrimitiveError> {
    let mut v = vec![];
    v.extend(min_length(value, 2));
    for (i, s) in value.iter().enumerate() {
        let sub = format!("[{i}].id");
        let valid = !s.id.is_empty()
            && s.id.starts_with(|c: char| c.is_ascii_uppercase())
            && s.id.chars().all(|c| c.is_ascii_alphanumeric());
        if !valid {
            v.push(PrimitiveError::naming_format_violation(
                format!("'{}' is not PascalCase", s.id),
                Some(sub),
                "pascal_case",
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
    let has_done = value
        .iter()
        .any(|s| matches!(s.semantic, Some(crate::entity::types::TaskSemantic::Done)));
    let has_non_done = value
        .iter()
        .any(|s| !matches!(s.semantic, Some(crate::entity::types::TaskSemantic::Done)));
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
