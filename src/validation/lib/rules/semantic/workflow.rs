use crate::entity::entities::workflow::{
    Step, TrackedEmbeddedWorkflow, TrackedReusableWorkflow, TrackedWorkflow,
};
use crate::error::primitive::PrimitiveError;

// ---------------------------------------------------------------------------
// Workflow
// ---------------------------------------------------------------------------

pub async fn depends_on_valid(e: &TrackedWorkflow) -> Vec<PrimitiveError> {
    let steps = match e.steps.get() {
        Some(s) => s,
        None => return vec![],
    };
    let mut violations = vec![];
    let step_ids: std::collections::HashSet<&str> = steps.keys().map(|s| s.as_str()).collect();
    for (step_id, step) in steps.iter() {
        match step {
            Step::Task { depends_on: Some(deps), .. }
            | Step::Relay { depends_on: Some(deps), .. }
            | Step::EmbeddedWorkflow { depends_on: Some(deps), .. } => {
                for dep in deps {
                    if !step_ids.contains(dep.as_str()) {
                        violations.push(PrimitiveError::illegal_dependency_reference(
                            "step does not exist",
                            format!(".{step_id}.depends_on"),
                            dep.clone(),
                        ));
                    }
                }
            }
            _ => {}
        }
    }
    violations
}

pub async fn on_reject_valid(e: &TrackedWorkflow) -> Vec<PrimitiveError> {
    let steps = match e.steps.get() {
        Some(s) => s,
        None => return vec![],
    };
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

pub async fn reviewing_state_required(e: &TrackedWorkflow) -> Vec<PrimitiveError> {
    let steps = match e.steps.get() {
        Some(s) => s,
        None => return vec![],
    };
    if !steps.values().any(|s| matches!(s, Step::Review { .. })) {
        return vec![];
    }
    let states = match e.states.get() {
        Some(s) => s,
        None => return vec![],
    };
    let has_reviewing = states.iter().any(|s| {
        matches!(s.semantic, Some(crate::entity::types::WorkflowSemantic::Reviewing))
    });
    if !has_reviewing {
        vec![PrimitiveError::workflow_graph_inconsistency(
            "workflow has Review steps but no state with Reviewing semantic",
            "missing_reviewing_semantic",
        )]
    } else {
        vec![]
    }
}

// ---------------------------------------------------------------------------
// ReusableWorkflow
// ---------------------------------------------------------------------------

pub async fn depends_on_valid_reusable(e: &TrackedReusableWorkflow) -> Vec<PrimitiveError> {
    let steps = match e.steps.get() {
        Some(s) => s,
        None => return vec![],
    };
    let mut violations = vec![];
    let step_ids: std::collections::HashSet<&str> = steps.keys().map(|s| s.as_str()).collect();
    for (step_id, step) in steps.iter() {
        match step {
            Step::Task { depends_on: Some(deps), .. }
            | Step::Relay { depends_on: Some(deps), .. }
            | Step::EmbeddedWorkflow { depends_on: Some(deps), .. } => {
                for dep in deps {
                    if !step_ids.contains(dep.as_str()) {
                        violations.push(PrimitiveError::illegal_dependency_reference(
                            "step does not exist",
                            format!(".{step_id}.depends_on"),
                            dep.clone(),
                        ));
                    }
                }
            }
            _ => {}
        }
    }
    violations
}

pub async fn on_reject_valid_reusable(e: &TrackedReusableWorkflow) -> Vec<PrimitiveError> {
    let steps = match e.steps.get() {
        Some(s) => s,
        None => return vec![],
    };
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

pub async fn reviewing_state_required_reusable(e: &TrackedReusableWorkflow) -> Vec<PrimitiveError> {
    let steps = match e.steps.get() {
        Some(s) => s,
        None => return vec![],
    };
    if !steps.values().any(|s| matches!(s, Step::Review { .. })) {
        return vec![];
    }
    let states = match e.states.get() {
        Some(s) => s,
        None => return vec![],
    };
    let has_reviewing = states.iter().any(|s| {
        matches!(s.semantic, Some(crate::entity::types::WorkflowSemantic::Reviewing))
    });
    if !has_reviewing {
        vec![PrimitiveError::workflow_graph_inconsistency(
            "workflow has Review steps but no state with Reviewing semantic",
            "missing_reviewing_semantic",
        )]
    } else {
        vec![]
    }
}

// ---------------------------------------------------------------------------
// EmbeddedWorkflow
// ---------------------------------------------------------------------------

pub async fn on_reject_valid_embedded(e: &TrackedEmbeddedWorkflow) -> Vec<PrimitiveError> {
    let steps = match e.steps.get() {
        Some(s) => s,
        None => return vec![],
    };
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

pub async fn reviewing_state_required_embedded(e: &TrackedEmbeddedWorkflow) -> Vec<PrimitiveError> {
    let steps = match e.steps.get() {
        Some(s) => s,
        None => return vec![],
    };
    if !steps.values().any(|s| matches!(s, Step::Review { .. })) {
        return vec![];
    }
    let states = match e.states.get() {
        Some(s) => s,
        None => return vec![],
    };
    let has_reviewing = states.iter().any(|s| {
        matches!(s.semantic, Some(crate::entity::types::WorkflowSemantic::Reviewing))
    });
    if !has_reviewing {
        vec![PrimitiveError::workflow_graph_inconsistency(
            "workflow has Review steps but no state with Reviewing semantic",
            "missing_reviewing_semantic",
        )]
    } else {
        vec![]
    }
}
