//! Structural, semantic, and cross-entity validation schemas for Workflow variants.

use super::{
    camel_case_id, non_empty_str, raci_structural, states_valid_workflow, x_prefix_keys,
    AnyCrossEntityRule, AnySemanticRule, AnyStructuralRule, RuleViolation, ValidationSchema,
};
use crate::entities::workflow::{
    EmbeddedWorkflow, ReusableWorkflow, Step, TrackedEmbeddedWorkflow, TrackedReusableWorkflow,
    TrackedWorkflow, Workflow,
};

fn opt_non_empty_str(value: &Option<String>) -> Vec<RuleViolation> {
    match value {
        None => vec![],
        Some(s) => non_empty_str(s),
    }
}

// ---------------------------------------------------------------------------
// Semantic rules shared by all workflow variants
// ---------------------------------------------------------------------------

/// For each Review step, `on_reject` must refer to an existing step id.
async fn depends_on_valid(e: &TrackedWorkflow) -> Vec<RuleViolation> {
    let steps = match e.steps.get() {
        Some(s) => s,
        None => return vec![],
    };
    let mut violations = vec![];
    let step_ids: std::collections::HashSet<&str> = steps.keys().map(|s| s.as_str()).collect();
    for (step_id, step) in steps.iter() {
        match step {
            Step::Task {
                depends_on: Some(deps),
                ..
            }
            | Step::Relay {
                depends_on: Some(deps),
                ..
            }
            | Step::EmbeddedWorkflow {
                depends_on: Some(deps),
                ..
            } => {
                for dep in deps {
                    if !step_ids.contains(dep.as_str()) {
                        violations.push(RuleViolation::sub(
                            format!(".{step_id}.depends_on"),
                            format!("step '{dep}' does not exist"),
                        ));
                    }
                }
            }
            _ => {}
        }
    }
    violations
}

/// For each Review step, `on_reject` must refer to an existing step id.
async fn on_reject_valid(e: &TrackedWorkflow) -> Vec<RuleViolation> {
    let steps = match e.steps.get() {
        Some(s) => s,
        None => return vec![],
    };
    let mut violations = vec![];
    let step_ids: std::collections::HashSet<&str> = steps.keys().map(|s| s.as_str()).collect();
    for (step_id, step) in steps.iter() {
        if let Step::Review { on_reject, .. } = step {
            if !step_ids.contains(on_reject.as_str()) {
                violations.push(RuleViolation::sub(
                    format!(".{step_id}.on_reject"),
                    format!("step '{on_reject}' does not exist"),
                ));
            }
        }
    }
    violations
}

/// If any Review step is present, the workflow must have a state with Reviewing semantic.
async fn reviewing_state_required(e: &TrackedWorkflow) -> Vec<RuleViolation> {
    let steps = match e.steps.get() {
        Some(s) => s,
        None => return vec![],
    };
    let has_review = steps.values().any(|s| matches!(s, Step::Review { .. }));
    if !has_review {
        return vec![];
    }
    let states = match e.states.get() {
        Some(s) => s,
        None => return vec![],
    };
    let has_reviewing = states
        .iter()
        .any(|s| matches!(s.semantic, Some(crate::types::WorkflowSemantic::Reviewing)));
    if !has_reviewing {
        vec![RuleViolation::field(
            "workflow has Review steps but no state with Reviewing semantic",
        )]
    } else {
        vec![]
    }
}

// ---------------------------------------------------------------------------
// ReusableWorkflow semantic rules (same structure)
// ---------------------------------------------------------------------------

async fn on_reject_valid_reusable(e: &TrackedReusableWorkflow) -> Vec<RuleViolation> {
    let steps = match e.steps.get() {
        Some(s) => s,
        None => return vec![],
    };
    let mut violations = vec![];
    let step_ids: std::collections::HashSet<&str> = steps.keys().map(|s| s.as_str()).collect();
    for (step_id, step) in steps.iter() {
        if let Step::Review { on_reject, .. } = step {
            if !step_ids.contains(on_reject.as_str()) {
                violations.push(RuleViolation::sub(
                    format!(".{step_id}.on_reject"),
                    format!("step '{on_reject}' does not exist"),
                ));
            }
        }
    }
    violations
}

async fn reviewing_state_required_reusable(e: &TrackedReusableWorkflow) -> Vec<RuleViolation> {
    let steps = match e.steps.get() {
        Some(s) => s,
        None => return vec![],
    };
    let has_review = steps.values().any(|s| matches!(s, Step::Review { .. }));
    if !has_review {
        return vec![];
    }
    let states = match e.states.get() {
        Some(s) => s,
        None => return vec![],
    };
    let has_reviewing = states
        .iter()
        .any(|s| matches!(s.semantic, Some(crate::types::WorkflowSemantic::Reviewing)));
    if !has_reviewing {
        vec![RuleViolation::field(
            "workflow has Review steps but no state with Reviewing semantic",
        )]
    } else {
        vec![]
    }
}

async fn depends_on_valid_reusable(e: &TrackedReusableWorkflow) -> Vec<RuleViolation> {
    let steps = match e.steps.get() {
        Some(s) => s,
        None => return vec![],
    };
    let mut violations = vec![];
    let step_ids: std::collections::HashSet<&str> = steps.keys().map(|s| s.as_str()).collect();
    for (step_id, step) in steps.iter() {
        match step {
            Step::Task {
                depends_on: Some(deps),
                ..
            }
            | Step::Relay {
                depends_on: Some(deps),
                ..
            }
            | Step::EmbeddedWorkflow {
                depends_on: Some(deps),
                ..
            } => {
                for dep in deps {
                    if !step_ids.contains(dep.as_str()) {
                        violations.push(RuleViolation::sub(
                            format!(".{step_id}.depends_on"),
                            format!("step '{dep}' does not exist"),
                        ));
                    }
                }
            }
            _ => {}
        }
    }
    violations
}

// ---------------------------------------------------------------------------
// EmbeddedWorkflow semantic rules
// ---------------------------------------------------------------------------

async fn on_reject_valid_embedded(e: &TrackedEmbeddedWorkflow) -> Vec<RuleViolation> {
    let steps = match e.steps.get() {
        Some(s) => s,
        None => return vec![],
    };
    let mut violations = vec![];
    let step_ids: std::collections::HashSet<&str> = steps.keys().map(|s| s.as_str()).collect();
    for (step_id, step) in steps.iter() {
        if let Step::Review { on_reject, .. } = step {
            if !step_ids.contains(on_reject.as_str()) {
                violations.push(RuleViolation::sub(
                    format!(".{step_id}.on_reject"),
                    format!("step '{on_reject}' does not exist"),
                ));
            }
        }
    }
    violations
}

async fn reviewing_state_required_embedded(e: &TrackedEmbeddedWorkflow) -> Vec<RuleViolation> {
    let steps = match e.steps.get() {
        Some(s) => s,
        None => return vec![],
    };
    let has_review = steps.values().any(|s| matches!(s, Step::Review { .. }));
    if !has_review {
        return vec![];
    }
    let states = match e.states.get() {
        Some(s) => s,
        None => return vec![],
    };
    let has_reviewing = states
        .iter()
        .any(|s| matches!(s.semantic, Some(crate::types::WorkflowSemantic::Reviewing)));
    if !has_reviewing {
        vec![RuleViolation::field(
            "workflow has Review steps but no state with Reviewing semantic",
        )]
    } else {
        vec![]
    }
}

// ---------------------------------------------------------------------------
// Schema builders
// ---------------------------------------------------------------------------

macro_rules! common_structural {
    ($E:ty, $Tracked:ty) => {{
        let mut structural: std::collections::HashMap<&'static str, Vec<AnyStructuralRule<$E>>> =
            std::collections::HashMap::new();

        structural.insert(
            "entity_ref",
            vec![Box::new(|e: &$Tracked| camel_case_id(&e.entity_ref))],
        );

        structural.insert(
            "name",
            vec![Box::new(|e: &$Tracked| {
                e.name.get().map(|v| non_empty_str(v)).unwrap_or_default()
            })],
        );

        structural.insert(
            "description",
            vec![Box::new(|e: &$Tracked| {
                e.description
                    .get()
                    .map(|v| opt_non_empty_str(v))
                    .unwrap_or_default()
            })],
        );

        structural.insert(
            "purpose",
            vec![Box::new(|e: &$Tracked| {
                e.purpose
                    .get()
                    .map(|v| non_empty_str(v))
                    .unwrap_or_default()
            })],
        );

        structural.insert(
            "raci",
            vec![Box::new(|e: &$Tracked| {
                e.raci.get().map(|v| raci_structural(v)).unwrap_or_default()
            })],
        );

        structural.insert(
            "states",
            vec![Box::new(|e: &$Tracked| {
                e.states
                    .get()
                    .map(|v| states_valid_workflow(v.as_slice()))
                    .unwrap_or_default()
            })],
        );

        structural.insert(
            "extensions",
            vec![Box::new(|e: &$Tracked| {
                e.extensions
                    .get()
                    .map(|v| x_prefix_keys(v))
                    .unwrap_or_default()
            })],
        );

        structural
    }};
}

pub fn workflow_validation_schema() -> ValidationSchema<Workflow> {
    let structural = common_structural!(Workflow, TrackedWorkflow);

    let mut semantic: std::collections::HashMap<&'static str, Vec<AnySemanticRule<Workflow>>> =
        std::collections::HashMap::new();

    semantic.insert(
        "steps",
        vec![
            Box::new(|e: &TrackedWorkflow| Box::pin(depends_on_valid(e))),
            Box::new(|e: &TrackedWorkflow| Box::pin(on_reject_valid(e))),
            Box::new(|e: &TrackedWorkflow| Box::pin(reviewing_state_required(e))),
        ],
    );

    let mut cross_entity: std::collections::HashMap<
        &'static str,
        Vec<AnyCrossEntityRule<Workflow>>,
    > = std::collections::HashMap::new();

    // Stubs
    cross_entity.insert(
        "steps",
        vec![
            Box::new(|_e: &TrackedWorkflow| Box::pin(async { vec![] })), // work_step_refs_exist
            Box::new(|_e: &TrackedWorkflow| Box::pin(async { vec![] })), // review_approver_roles_exist
        ],
    );

    cross_entity.insert(
        "raci",
        vec![Box::new(|_e: &TrackedWorkflow| Box::pin(async { vec![] }))], // raci_roles_exist
    );

    ValidationSchema {
        structural,
        semantic,
        cross_entity,
    }
}

pub fn reusable_workflow_validation_schema() -> ValidationSchema<ReusableWorkflow> {
    let structural = common_structural!(ReusableWorkflow, TrackedReusableWorkflow);

    let mut semantic: std::collections::HashMap<
        &'static str,
        Vec<AnySemanticRule<ReusableWorkflow>>,
    > = std::collections::HashMap::new();

    semantic.insert(
        "steps",
        vec![
            Box::new(|e: &TrackedReusableWorkflow| Box::pin(depends_on_valid_reusable(e))),
            Box::new(|e: &TrackedReusableWorkflow| Box::pin(on_reject_valid_reusable(e))),
            Box::new(|e: &TrackedReusableWorkflow| Box::pin(reviewing_state_required_reusable(e))),
        ],
    );

    let mut cross_entity: std::collections::HashMap<
        &'static str,
        Vec<AnyCrossEntityRule<ReusableWorkflow>>,
    > = std::collections::HashMap::new();

    cross_entity.insert(
        "steps",
        vec![
            Box::new(|_e: &TrackedReusableWorkflow| Box::pin(async { vec![] })),
            Box::new(|_e: &TrackedReusableWorkflow| Box::pin(async { vec![] })),
            // no_relay_in_tree stub
            Box::new(|_e: &TrackedReusableWorkflow| Box::pin(async { vec![] })),
        ],
    );

    cross_entity.insert(
        "raci",
        vec![Box::new(|_e: &TrackedReusableWorkflow| {
            Box::pin(async { vec![] })
        })],
    );

    ValidationSchema {
        structural,
        semantic,
        cross_entity,
    }
}

pub fn embedded_workflow_validation_schema() -> ValidationSchema<EmbeddedWorkflow> {
    let mut structural: std::collections::HashMap<
        &'static str,
        Vec<AnyStructuralRule<EmbeddedWorkflow>>,
    > = std::collections::HashMap::new();

    structural.insert(
        "entity_ref",
        vec![Box::new(|e: &TrackedEmbeddedWorkflow| {
            camel_case_id(&e.entity_ref)
        })],
    );

    structural.insert(
        "name",
        vec![Box::new(|e: &TrackedEmbeddedWorkflow| {
            e.name.get().map(|v| non_empty_str(v)).unwrap_or_default()
        })],
    );

    structural.insert(
        "description",
        vec![Box::new(|e: &TrackedEmbeddedWorkflow| {
            e.description
                .get()
                .map(|v| opt_non_empty_str(v))
                .unwrap_or_default()
        })],
    );

    structural.insert(
        "purpose",
        vec![Box::new(|e: &TrackedEmbeddedWorkflow| {
            e.purpose
                .get()
                .map(|v| non_empty_str(v))
                .unwrap_or_default()
        })],
    );

    structural.insert(
        "raci",
        vec![Box::new(|e: &TrackedEmbeddedWorkflow| {
            e.raci
                .get()
                .map(|opt_raci| {
                    opt_raci
                        .as_ref()
                        .map(|r| raci_structural(r))
                        .unwrap_or_default()
                })
                .unwrap_or_default()
        })],
    );

    structural.insert(
        "states",
        vec![Box::new(|e: &TrackedEmbeddedWorkflow| {
            e.states
                .get()
                .map(|v| states_valid_workflow(v.as_slice()))
                .unwrap_or_default()
        })],
    );

    structural.insert(
        "extensions",
        vec![Box::new(|e: &TrackedEmbeddedWorkflow| {
            e.extensions
                .get()
                .map(|v| x_prefix_keys(v))
                .unwrap_or_default()
        })],
    );

    let mut semantic: std::collections::HashMap<
        &'static str,
        Vec<AnySemanticRule<EmbeddedWorkflow>>,
    > = std::collections::HashMap::new();

    semantic.insert(
        "steps",
        vec![
            Box::new(|e: &TrackedEmbeddedWorkflow| Box::pin(on_reject_valid_embedded(e))),
            Box::new(|e: &TrackedEmbeddedWorkflow| Box::pin(reviewing_state_required_embedded(e))),
        ],
    );

    let mut cross_entity: std::collections::HashMap<
        &'static str,
        Vec<AnyCrossEntityRule<EmbeddedWorkflow>>,
    > = std::collections::HashMap::new();

    cross_entity.insert(
        "steps",
        vec![
            Box::new(|_e: &TrackedEmbeddedWorkflow| Box::pin(async { vec![] })),
            Box::new(|_e: &TrackedEmbeddedWorkflow| Box::pin(async { vec![] })),
        ],
    );

    cross_entity.insert(
        "raci",
        vec![Box::new(|_e: &TrackedEmbeddedWorkflow| {
            Box::pin(async { vec![] })
        })],
    );

    ValidationSchema {
        structural,
        semantic,
        cross_entity,
    }
}
