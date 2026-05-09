use crate::{
    entity::{
        entities::workflow::{
            Step, TrackedEmbeddedWorkflow, TrackedReusableWorkflow, TrackedWorkflow,
        },
        types::{WorkflowSemantic, WorkflowStateEntry},
    },
    error::primitive::PrimitiveError,
};

// ---------------------------------------------------------------------------
// Workflow
// ---------------------------------------------------------------------------

fn check_depends_on(steps: &indexmap::IndexMap<String, Step>) -> Vec<PrimitiveError> {
    let step_position: std::collections::HashMap<&str, usize> = steps
        .keys()
        .enumerate()
        .map(|(i, k)| (k.as_str(), i))
        .collect();
    let mut violations = vec![];
    for (step_id, step) in steps.iter() {
        let deps = match step {
            Step::Task {
                depends_on: Some(d),
                ..
            }
            | Step::Relay {
                depends_on: Some(d),
                ..
            }
            | Step::EmbeddedWorkflow {
                depends_on: Some(d),
                ..
            } => d,
            _ => continue,
        };
        let current_pos = step_position[step_id.as_str()];
        for (i, dep) in deps.iter().enumerate() {
            match step_position.get(dep.as_str()) {
                None => violations.push(PrimitiveError::illegal_dependency_reference(
                    "step does not exist",
                    format!(".{step_id}.depends_on[{i}]"),
                    dep.clone(),
                )),
                Some(&dep_pos) if dep_pos >= current_pos => {
                    violations.push(PrimitiveError::illegal_dependency_reference(
                        "depends_on must reference a step that appears earlier in the workflow",
                        format!(".{step_id}.depends_on[{i}]"),
                        dep.clone(),
                    ));
                }
                _ => {}
            }
        }
    }
    violations
}

pub async fn depends_on_valid(e: &TrackedWorkflow) -> Vec<PrimitiveError> {
    match e.steps.get() {
        Some(s) => check_depends_on(s),
        None => vec![],
    }
}

fn check_on_reject(steps: &indexmap::IndexMap<String, Step>) -> Vec<PrimitiveError> {
    let mut violations = vec![];
    let step_ids: std::collections::HashSet<&str> = steps.keys().map(|s| s.as_str()).collect();
    for (step_id, step) in steps.iter() {
        if let Step::Review { on_reject, .. } = step {
            if !step_ids.contains(on_reject.as_str()) {
                violations.push(PrimitiveError::invalid_on_reject_target(
                    "on_reject target does not exist",
                    format!(".{step_id}.on_reject"),
                    on_reject.clone(),
                ));
            }
        }
    }
    violations
}

pub async fn on_reject_valid(e: &TrackedWorkflow) -> Vec<PrimitiveError> {
    match e.steps.get() {
        Some(s) => check_on_reject(s),
        None => vec![],
    }
}

/// If any step is a `Review`, the states list must include one with
/// `WorkflowSemantic::Reviewing`. Returns no violations when there are
/// no Review steps, regardless of state shape.
fn check_reviewing_state(
    steps: &indexmap::IndexMap<String, Step>,
    states: &[WorkflowStateEntry],
) -> Vec<PrimitiveError> {
    if !steps.values().any(|s| matches!(s, Step::Review { .. })) {
        return vec![];
    }
    let has_reviewing = states
        .iter()
        .any(|s| matches!(s.semantic, Some(WorkflowSemantic::Reviewing)));
    if has_reviewing {
        vec![]
    } else {
        vec![PrimitiveError::workflow_graph_inconsistency(
            "workflow has Review steps but no state with Reviewing semantic",
            "missing_reviewing_semantic",
        )]
    }
}

pub async fn reviewing_state_required(e: &TrackedWorkflow) -> Vec<PrimitiveError> {
    match (e.steps.get(), e.states.get()) {
        (Some(steps), Some(states)) => check_reviewing_state(steps, states),
        _ => vec![],
    }
}

// ---------------------------------------------------------------------------
// ReusableWorkflow
// ---------------------------------------------------------------------------

pub async fn depends_on_valid_reusable(e: &TrackedReusableWorkflow) -> Vec<PrimitiveError> {
    match e.steps.get() {
        Some(s) => check_depends_on(s),
        None => vec![],
    }
}

pub async fn on_reject_valid_reusable(e: &TrackedReusableWorkflow) -> Vec<PrimitiveError> {
    match e.steps.get() {
        Some(s) => check_on_reject(s),
        None => vec![],
    }
}

pub async fn reviewing_state_required_reusable(e: &TrackedReusableWorkflow) -> Vec<PrimitiveError> {
    match (e.steps.get(), e.states.get()) {
        (Some(steps), Some(states)) => check_reviewing_state(steps, states),
        _ => vec![],
    }
}

// ---------------------------------------------------------------------------
// EmbeddedWorkflow
// ---------------------------------------------------------------------------

pub async fn depends_on_valid_embedded(e: &TrackedEmbeddedWorkflow) -> Vec<PrimitiveError> {
    match e.steps.get() {
        Some(s) => check_depends_on(s),
        None => vec![],
    }
}

pub async fn on_reject_valid_embedded(e: &TrackedEmbeddedWorkflow) -> Vec<PrimitiveError> {
    match e.steps.get() {
        Some(s) => check_on_reject(s),
        None => vec![],
    }
}

pub async fn reviewing_state_required_embedded(e: &TrackedEmbeddedWorkflow) -> Vec<PrimitiveError> {
    match (e.steps.get(), e.states.get()) {
        (Some(steps), Some(states)) => check_reviewing_state(steps, states),
        _ => vec![],
    }
}

#[cfg(test)]
mod tests {
    //! Unit coverage for the pure step-graph rules. Functional tests
    //! pin the rule binding (a specific bad workflow surfaces a
    //! specific error); these cover input-shape edge cases that don't
    //! belong end-to-end.

    use indexmap::IndexMap;

    use super::*;
    use crate::entity::{
        entities::{
            relay::Relay,
            role::Role,
            task::Task,
            workflow::{EmbeddedWorkflow, Workflow},
        },
        EntityRef, WorkflowParent,
    };

    fn workflow_parent() -> WorkflowParent {
        WorkflowParent::Workflow(EntityRef::<Workflow>::new("WF"))
    }

    fn task_step(id: &str, depends_on: Option<Vec<&str>>) -> Step {
        Step::Task {
            entity_ref: EntityRef::<Task, _>::with_parent(id, workflow_parent()),
            depends_on: depends_on.map(|v| v.into_iter().map(String::from).collect()),
        }
    }

    fn relay_step(id: &str, depends_on: Option<Vec<&str>>) -> Step {
        Step::Relay {
            entity_ref: EntityRef::<Relay, _>::with_parent(id, workflow_parent()),
            depends_on: depends_on.map(|v| v.into_iter().map(String::from).collect()),
        }
    }

    fn embedded_step(id: &str, depends_on: Option<Vec<&str>>) -> Step {
        Step::EmbeddedWorkflow {
            entity_ref: EntityRef::<EmbeddedWorkflow, _>::with_parent(id, workflow_parent()),
            depends_on: depends_on.map(|v| v.into_iter().map(String::from).collect()),
        }
    }

    fn review_step(approver: &str, on_reject: &str) -> Step {
        Step::Review {
            approver: vec![EntityRef::<Role>::new(approver)],
            on_reject: on_reject.to_string(),
        }
    }

    fn steps(entries: Vec<(&str, Step)>) -> IndexMap<String, Step> {
        entries
            .into_iter()
            .map(|(k, s)| (k.to_string(), s))
            .collect()
    }

    // -----------------------------------------------------------------------
    // check_depends_on
    // -----------------------------------------------------------------------

    #[test]
    fn depends_on_empty_steps_no_violations() {
        assert!(check_depends_on(&IndexMap::new()).is_empty());
    }

    #[test]
    fn depends_on_no_dependencies_no_violations() {
        let s = steps(vec![
            ("A", task_step("A", None)),
            ("B", task_step("B", None)),
        ]);
        assert!(check_depends_on(&s).is_empty());
    }

    #[test]
    fn depends_on_valid_backward_reference() {
        let s = steps(vec![
            ("A", task_step("A", None)),
            ("B", task_step("B", Some(vec!["A"]))),
        ]);
        assert!(check_depends_on(&s).is_empty());
    }

    #[test]
    fn depends_on_unknown_target_violates() {
        let s = steps(vec![("A", task_step("A", Some(vec!["Phantom"])))]);
        let v = check_depends_on(&s);
        assert_eq!(v.len(), 1);
        assert!(matches!(
            v[0],
            PrimitiveError::IllegalDependencyReference { .. }
        ));
    }

    #[test]
    fn depends_on_self_loop_violates_as_forward_reference() {
        // Self-loop: A.depends_on = [A]. Position rule treats A's own
        // position as not "earlier than itself" — flagged.
        let s = steps(vec![("A", task_step("A", Some(vec!["A"])))]);
        let v = check_depends_on(&s);
        assert_eq!(v.len(), 1);
        assert!(matches!(
            v[0],
            PrimitiveError::IllegalDependencyReference { .. }
        ));
    }

    #[test]
    fn depends_on_forward_reference_violates() {
        // A depends on B, but B appears later — forward reference.
        let s = steps(vec![
            ("A", task_step("A", Some(vec!["B"]))),
            ("B", task_step("B", None)),
        ]);
        let v = check_depends_on(&s);
        assert_eq!(v.len(), 1);
        assert!(matches!(
            v[0],
            PrimitiveError::IllegalDependencyReference { .. }
        ));
    }

    #[test]
    fn depends_on_branching_valid_when_all_earlier() {
        let s = steps(vec![
            ("A", task_step("A", None)),
            ("B", task_step("B", None)),
            ("C", task_step("C", Some(vec!["A", "B"]))),
        ]);
        assert!(check_depends_on(&s).is_empty());
    }

    #[test]
    fn depends_on_collects_multiple_violations() {
        let s = steps(vec![
            ("A", task_step("A", Some(vec!["Phantom"]))),
            ("B", task_step("B", Some(vec!["A", "C"]))),
            ("C", task_step("C", None)),
        ]);
        // A.depends_on=Phantom (unknown) → 1
        // B.depends_on[0]=A is earlier → ok
        // B.depends_on[1]=C is later → 1
        let v = check_depends_on(&s);
        assert_eq!(v.len(), 2);
    }

    #[test]
    fn depends_on_applies_to_all_step_kinds() {
        // Same earlier-position rule applies whether the depending
        // step is Task, Relay, or EmbeddedWorkflow.
        let s = steps(vec![
            ("A", task_step("A", None)),
            ("B", relay_step("B", Some(vec!["A"]))),
            ("C", embedded_step("C", Some(vec!["A", "B"]))),
        ]);
        assert!(check_depends_on(&s).is_empty());
    }

    #[test]
    fn depends_on_review_steps_skipped() {
        // Review steps don't carry depends_on; the rule must just skip
        // them rather than panic or surface a spurious violation.
        let s = steps(vec![
            ("A", task_step("A", None)),
            ("R", review_step("approver", "A")),
        ]);
        assert!(check_depends_on(&s).is_empty());
    }

    // -----------------------------------------------------------------------
    // check_on_reject
    // -----------------------------------------------------------------------

    #[test]
    fn on_reject_no_review_steps_no_violations() {
        let s = steps(vec![
            ("A", task_step("A", None)),
            ("B", task_step("B", None)),
        ]);
        assert!(check_on_reject(&s).is_empty());
    }

    #[test]
    fn on_reject_target_exists() {
        let s = steps(vec![
            ("A", task_step("A", None)),
            ("R", review_step("approver", "A")),
        ]);
        assert!(check_on_reject(&s).is_empty());
    }

    #[test]
    fn on_reject_unknown_target_violates() {
        let s = steps(vec![("R", review_step("approver", "Phantom"))]);
        let v = check_on_reject(&s);
        assert_eq!(v.len(), 1);
        assert!(matches!(v[0], PrimitiveError::InvalidOnRejectTarget { .. }));
    }

    #[test]
    fn on_reject_self_target_is_accepted() {
        // The rule only checks existence of the target step; pointing
        // a Review at itself is structurally valid as far as this
        // rule is concerned (any cycle semantics are outside scope).
        let s = steps(vec![("R", review_step("approver", "R"))]);
        assert!(check_on_reject(&s).is_empty());
    }

    #[test]
    fn on_reject_collects_violations_from_multiple_review_steps() {
        let s = steps(vec![
            ("A", task_step("A", None)),
            ("R1", review_step("approver", "A")),        // ok
            ("R2", review_step("approver", "Phantom1")), // bad
            ("R3", review_step("approver", "Phantom2")), // bad
        ]);
        let v = check_on_reject(&s);
        assert_eq!(v.len(), 2);
    }

    // -----------------------------------------------------------------------
    // check_reviewing_state
    // -----------------------------------------------------------------------

    fn state(id: &str, semantic: Option<WorkflowSemantic>) -> WorkflowStateEntry {
        WorkflowStateEntry {
            id: id.to_string(),
            description: String::new(),
            semantic,
        }
    }

    #[test]
    fn reviewing_state_no_review_step_no_violations() {
        // Without a Review step, the rule never fires — even if the
        // states list lacks a Reviewing-semantic state.
        let s = steps(vec![("A", task_step("A", None))]);
        let states = vec![
            state("InProgress", None),
            state("Done", Some(WorkflowSemantic::Done)),
        ];
        assert!(check_reviewing_state(&s, &states).is_empty());
    }

    #[test]
    fn reviewing_state_required_when_review_step_present() {
        let s = steps(vec![
            ("A", task_step("A", None)),
            ("R", review_step("approver", "A")),
        ]);
        let states = vec![
            state("InProgress", None),
            state("InReview", Some(WorkflowSemantic::Reviewing)),
            state("Done", Some(WorkflowSemantic::Done)),
        ];
        assert!(check_reviewing_state(&s, &states).is_empty());
    }

    #[test]
    fn reviewing_state_missing_when_review_step_present_violates() {
        let s = steps(vec![
            ("A", task_step("A", None)),
            ("R", review_step("approver", "A")),
        ]);
        let states = vec![
            state("InProgress", None),
            state("Done", Some(WorkflowSemantic::Done)),
        ];
        let v = check_reviewing_state(&s, &states);
        assert_eq!(v.len(), 1);
        assert!(matches!(
            v[0],
            PrimitiveError::WorkflowGraphInconsistency { .. }
        ));
    }

    #[test]
    fn reviewing_state_one_review_among_many_steps_still_fires_rule() {
        // A single Review step is enough to require the Reviewing
        // semantic — the rule doesn't count multiplicities.
        let s = steps(vec![
            ("A", task_step("A", None)),
            ("B", task_step("B", Some(vec!["A"]))),
            ("R", review_step("approver", "B")),
        ]);
        let states = vec![
            state("InProgress", None),
            state("Done", Some(WorkflowSemantic::Done)),
        ];
        let v = check_reviewing_state(&s, &states);
        assert_eq!(v.len(), 1);
    }
}
