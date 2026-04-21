use super::primitives::min_length;
use crate::{entity::types::WorkflowStateEntry, error::primitive::PrimitiveError};

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::primitive::PrimitiveError;

    fn make_state(
        id: &str,
        semantic: Option<crate::entity::types::WorkflowSemantic>,
    ) -> WorkflowStateEntry {
        WorkflowStateEntry {
            id: id.to_string(),
            description: "d".to_string(),
            semantic,
        }
    }

    #[test]
    fn valid_states() {
        let states = vec![
            make_state("Draft", None),
            make_state("Done", Some(crate::entity::types::WorkflowSemantic::Done)),
        ];
        assert!(states_valid_workflow(&states).is_empty());
    }

    #[test]
    fn requires_min_2() {
        let states = vec![make_state(
            "Done",
            Some(crate::entity::types::WorkflowSemantic::Done),
        )];
        assert!(!states_valid_workflow(&states).is_empty());
    }

    #[test]
    fn requires_done_semantic() {
        let states = vec![make_state("Draft", None), make_state("Active", None)];
        let v = states_valid_workflow(&states);
        assert!(v
            .iter()
            .any(|e| matches!(e, PrimitiveError::WorkflowGraphInconsistency { .. })));
    }

    #[test]
    fn rejects_duplicate_ids() {
        let states = vec![
            make_state("Draft", None),
            make_state("Draft", Some(crate::entity::types::WorkflowSemantic::Done)),
        ];
        let v = states_valid_workflow(&states);
        assert!(v
            .iter()
            .any(|e| matches!(e, PrimitiveError::DuplicateEntryViolation { .. })));
    }

    #[test]
    fn rejects_lowercase_id() {
        let states = vec![
            make_state("draft", None),
            make_state("Done", Some(crate::entity::types::WorkflowSemantic::Done)),
        ];
        let v = states_valid_workflow(&states);
        assert!(v.iter().any(|e| match e {
            PrimitiveError::NamingFormatViolation { sub_path, .. } => {
                sub_path.as_ref().map(|p| p.contains("id")).unwrap_or(false)
            }
            _ => false,
        }));
    }
}
