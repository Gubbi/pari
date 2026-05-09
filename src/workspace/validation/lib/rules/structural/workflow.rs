use indexmap::IndexMap;

use super::primitives::{min_length, pascal_case};
use crate::{
    entity::{entities::workflow::Step, types::WorkflowStateEntry},
    error::primitive::PrimitiveError,
};

/// Each `steps` map key must be PascalCase.
pub fn step_keys_pascal_case(steps: &IndexMap<String, Step>) -> Vec<PrimitiveError> {
    let mut v = vec![];
    for key in steps.keys() {
        v.extend(pascal_case(key));
    }
    v
}

/// State list validation for workflow states:
/// - At least 2 states
/// - All ids are PascalCase
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
    //! Unit coverage for workflow structural rules. Functional tests
    //! pin specific failures via insert paths; these enumerate the
    //! pure logic's input shapes directly.

    use super::*;
    use crate::entity::{
        entities::{role::Role, task::Task, workflow::Workflow},
        types::WorkflowSemantic,
        EntityRef, WorkflowParent,
    };

    fn state(id: &str, semantic: Option<WorkflowSemantic>) -> WorkflowStateEntry {
        WorkflowStateEntry {
            id: id.to_string(),
            description: String::new(),
            semantic,
        }
    }

    // -----------------------------------------------------------------------
    // states_valid_workflow
    // -----------------------------------------------------------------------

    fn canonical_states() -> Vec<WorkflowStateEntry> {
        vec![
            state("InProgress", None),
            state("Done", Some(WorkflowSemantic::Done)),
        ]
    }

    #[test]
    fn states_valid_canonical_passes() {
        assert!(states_valid_workflow(&canonical_states()).is_empty());
    }

    #[test]
    fn states_valid_empty_violates_min_length() {
        let v = states_valid_workflow(&[]);
        assert!(
            v.iter()
                .any(|e| matches!(e, PrimitiveError::MalformedCollectionValue { .. })),
            "expected min-length violation, got: {v:?}"
        );
    }

    #[test]
    fn states_valid_single_state_violates_min_length() {
        // min_length is 2; one state can't satisfy both Done and non-Done.
        let v = states_valid_workflow(&[state("Done", Some(WorkflowSemantic::Done))]);
        assert!(v
            .iter()
            .any(|e| matches!(e, PrimitiveError::MalformedCollectionValue { .. })));
    }

    #[test]
    fn states_valid_missing_done_semantic_violates() {
        let states = vec![state("A", None), state("B", None)];
        let v = states_valid_workflow(&states);
        assert!(
            v.iter().any(|e| matches!(
                e,
                PrimitiveError::WorkflowGraphInconsistency { reason, .. }
                    if reason == "missing_done_semantic"
            )),
            "expected missing_done_semantic, got: {v:?}"
        );
    }

    #[test]
    fn states_valid_all_done_violates() {
        let states = vec![
            state("Done", Some(WorkflowSemantic::Done)),
            state("AlsoDone", Some(WorkflowSemantic::Done)),
        ];
        let v = states_valid_workflow(&states);
        assert!(
            v.iter().any(|e| matches!(
                e,
                PrimitiveError::WorkflowGraphInconsistency { reason, .. }
                    if reason == "all_done_states"
            )),
            "expected all_done_states, got: {v:?}"
        );
    }

    #[test]
    fn states_valid_non_pascal_id_violates() {
        let states = vec![
            state("in-progress", None),
            state("Done", Some(WorkflowSemantic::Done)),
        ];
        let v = states_valid_workflow(&states);
        assert!(
            v.iter()
                .any(|e| matches!(e, PrimitiveError::NamingFormatViolation { .. })),
            "expected naming violation, got: {v:?}"
        );
    }

    #[test]
    fn states_valid_id_with_digit_first_violates() {
        let states = vec![
            state("1Step", None),
            state("Done", Some(WorkflowSemantic::Done)),
        ];
        assert!(states_valid_workflow(&states)
            .iter()
            .any(|e| matches!(e, PrimitiveError::NamingFormatViolation { .. })));
    }

    #[test]
    fn states_valid_duplicate_ids_violates() {
        let states = vec![
            state("Same", None),
            state("Same", Some(WorkflowSemantic::Done)),
        ];
        let v = states_valid_workflow(&states);
        assert!(
            v.iter()
                .any(|e| matches!(e, PrimitiveError::DuplicateEntryViolation { .. })),
            "expected duplicate violation, got: {v:?}"
        );
    }

    #[test]
    fn states_valid_collects_multiple_violations() {
        // Empty id (naming) + single entry (min length) + all-Done.
        let states = vec![state("", Some(WorkflowSemantic::Done))];
        let v = states_valid_workflow(&states);
        // Expect at least 3 issues: min_length, naming, all_done_states.
        assert!(
            v.len() >= 3,
            "expected ≥3 violations, got {} ({v:?})",
            v.len()
        );
    }

    // -----------------------------------------------------------------------
    // step_keys_pascal_case
    // -----------------------------------------------------------------------

    fn task_step(id: &str) -> Step {
        Step::Task {
            entity_ref: EntityRef::<Task, _>::with_parent(
                id,
                WorkflowParent::Workflow(EntityRef::<Workflow>::new("WF")),
            ),
            depends_on: None,
        }
    }

    fn review_step(approver: &str, on_reject: &str) -> Step {
        Step::Review {
            approver: vec![EntityRef::<Role>::new(approver)],
            on_reject: on_reject.to_string(),
        }
    }

    fn make_steps(entries: Vec<(&str, Step)>) -> IndexMap<String, Step> {
        entries
            .into_iter()
            .map(|(k, s)| (k.to_string(), s))
            .collect()
    }

    #[test]
    fn step_keys_pascal_case_empty_passes() {
        assert!(step_keys_pascal_case(&IndexMap::new()).is_empty());
    }

    #[test]
    fn step_keys_pascal_case_all_valid_passes() {
        let s = make_steps(vec![
            ("Design", task_step("Design")),
            ("Review", review_step("approver", "Design")),
        ]);
        assert!(step_keys_pascal_case(&s).is_empty());
    }

    #[test]
    fn step_keys_pascal_case_kebab_violates() {
        let s = make_steps(vec![("design-step", task_step("design-step"))]);
        let v = step_keys_pascal_case(&s);
        assert!(!v.is_empty(), "expected naming violation");
    }

    #[test]
    fn step_keys_pascal_case_lowercase_first_violates() {
        let s = make_steps(vec![("design", task_step("design"))]);
        assert!(!step_keys_pascal_case(&s).is_empty());
    }

    #[test]
    fn step_keys_pascal_case_collects_per_bad_key() {
        let s = make_steps(vec![
            ("Design", task_step("Design")),           // ok
            ("review-step", task_step("review-step")), // bad
            ("third", task_step("third")),             // bad
        ]);
        let v = step_keys_pascal_case(&s);
        assert_eq!(v.len(), 2);
    }
}
