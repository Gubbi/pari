use std::collections::HashSet;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::schema::context::RepoContext;
use crate::schema::types::{HooksMap, Raci, Step, WorkflowSemantic, WorkflowStateEntry};
use crate::schema::validation::{is_camel_case, validate_hooks_map, validate_raci, ValidationError};

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct Workflow {
    #[schemars(regex(pattern = r"^[A-Z][A-Za-z0-9]*$"))]
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub purpose: String,
    pub accountability: Raci,
    #[schemars(length(min = 1))]
    pub steps: Vec<Step>,
    #[schemars(length(min = 2))]
    pub states: Vec<WorkflowStateEntry>,
    pub hooks: Option<HooksMap>,
    pub guidance: Option<String>,
}

pub fn validate(workflow: &Workflow, ctx: &RepoContext) -> Vec<ValidationError> {
    let mut errors = Vec::new();

    errors.extend(validate_structural(workflow));
    errors.extend(validate_states_semantic(workflow));
    errors.extend(validate_step_id_uniqueness(workflow));
    errors.extend(validate_state_id_uniqueness(workflow));
    errors.extend(validate_review_step_on_reject(workflow));
    errors.extend(validate_work_step_depends_on(workflow));
    errors.extend(validate_referential_integrity(workflow, ctx));

    errors
}

fn validate_structural(workflow: &Workflow) -> Vec<ValidationError> {
    let mut errors = Vec::new();

    if !is_camel_case(&workflow.id) {
        errors.push(ValidationError {
            path: "id".to_string(),
            message: format!("id must be CamelCase, got '{}'", workflow.id),
        });
    }

    if workflow.steps.is_empty() {
        errors.push(ValidationError {
            path: "steps".to_string(),
            message: "steps must have at least one item".to_string(),
        });
    }

    for (i, step) in workflow.steps.iter().enumerate() {
        if !is_camel_case(step.id()) {
            errors.push(ValidationError {
                path: format!("steps[{}].id", i),
                message: format!("id must be CamelCase, got '{}'", step.id()),
            });
        }
    }

    if workflow.states.len() < 2 {
        errors.push(ValidationError {
            path: "states".to_string(),
            message: format!(
                "states must have at least 2 entries, got {}",
                workflow.states.len()
            ),
        });
    }

    for (i, state) in workflow.states.iter().enumerate() {
        if !is_camel_case(&state.id) {
            errors.push(ValidationError {
                path: format!("states[{}].id", i),
                message: format!("id must be CamelCase, got '{}'", state.id),
            });
        }
    }

    errors
}

fn validate_states_semantic(workflow: &Workflow) -> Vec<ValidationError> {
    let mut errors = Vec::new();

    let has_complete = workflow
        .states
        .iter()
        .any(|s| s.semantic == Some(WorkflowSemantic::Complete));
    let has_non_complete = workflow
        .states
        .iter()
        .any(|s| s.semantic != Some(WorkflowSemantic::Complete));

    if !has_complete {
        errors.push(ValidationError {
            path: "states".to_string(),
            message: "states must include at least one entry with semantic: complete".to_string(),
        });
    }

    if !has_non_complete {
        errors.push(ValidationError {
            path: "states".to_string(),
            message: "states must include at least one entry without semantic: complete".to_string(),
        });
    }

    // If any ReviewStep is present, at least one state must have semantic: reviewing
    let has_review_step = workflow.steps.iter().any(|s| matches!(s, Step::Review(_)));
    if has_review_step {
        let has_reviewing = workflow
            .states
            .iter()
            .any(|s| s.semantic == Some(WorkflowSemantic::Reviewing));
        if !has_reviewing {
            errors.push(ValidationError {
                path: "states".to_string(),
                message: "workflow has ReviewSteps but no state with semantic: reviewing"
                    .to_string(),
            });
        }
    }

    errors
}

fn validate_step_id_uniqueness(workflow: &Workflow) -> Vec<ValidationError> {
    let mut errors = Vec::new();
    let mut seen: HashSet<&str> = HashSet::new();

    for (i, step) in workflow.steps.iter().enumerate() {
        if !seen.insert(step.id()) {
            errors.push(ValidationError {
                path: format!("steps[{}].id", i),
                message: format!("duplicate step id '{}'", step.id()),
            });
        }
    }

    errors
}

fn validate_state_id_uniqueness(workflow: &Workflow) -> Vec<ValidationError> {
    let mut errors = Vec::new();
    let mut seen: HashSet<&str> = HashSet::new();

    for (i, state) in workflow.states.iter().enumerate() {
        if !seen.insert(state.id.as_str()) {
            errors.push(ValidationError {
                path: format!("states[{}].id", i),
                message: format!("duplicate state id '{}'", state.id),
            });
        }
    }

    errors
}

fn validate_review_step_on_reject(workflow: &Workflow) -> Vec<ValidationError> {
    let mut errors = Vec::new();

    for (i, step) in workflow.steps.iter().enumerate() {
        if let Step::Review(rs) = step {
            let step_ids_before: HashSet<&str> =
                workflow.steps[..i].iter().map(|s| s.id()).collect();

            if !step_ids_before.contains(rs.on_reject.as_str()) {
                // Check if it exists at all
                let exists_anywhere = workflow
                    .steps
                    .iter()
                    .any(|s| s.id() == rs.on_reject.as_str());
                let message = if exists_anywhere {
                    format!(
                        "on_reject '{}' must reference an earlier step, not a later one",
                        rs.on_reject
                    )
                } else {
                    format!("on_reject '{}' references unknown step", rs.on_reject)
                };
                errors.push(ValidationError {
                    path: format!("steps[{}].on_reject", i),
                    message,
                });
            }
        }
    }

    errors
}

fn validate_work_step_depends_on(workflow: &Workflow) -> Vec<ValidationError> {
    let mut errors = Vec::new();
    let all_ids: HashSet<&str> = workflow.steps.iter().map(|s| s.id()).collect();

    for (i, step) in workflow.steps.iter().enumerate() {
        if let Step::Work(ws) = step {
            if let Some(depends_on) = &ws.depends_on {
                for dep in depends_on {
                    if !all_ids.contains(dep.as_str()) {
                        errors.push(ValidationError {
                            path: format!("steps[{}].depends_on", i),
                            message: format!("depends_on references unknown step '{}'", dep),
                        });
                    }
                }
            }
        }
    }

    errors
}

fn validate_referential_integrity(
    workflow: &Workflow,
    ctx: &RepoContext,
) -> Vec<ValidationError> {
    let mut errors = Vec::new();

    errors.extend(validate_raci(&workflow.accountability, "accountability", ctx));

    if let Some(hooks) = &workflow.hooks {
        errors.extend(validate_hooks_map(hooks, "hooks", ctx));
    }

    errors
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::types::{
        HookInvocation, HookInvocationValue, ReviewStep, Step, WorkStep, WorkflowSemantic,
        WorkflowStateEntry,
    };
    use std::collections::HashMap;

    fn make_ctx() -> RepoContext {
        let mut ctx = RepoContext::new();
        ctx.role_ids.insert("eng-lead".to_string());
        ctx.role_ids.insert("pm".to_string());
        ctx.hook_ids.insert("NotifySlack".to_string());
        ctx
    }

    fn base_raci() -> Raci {
        Raci {
            responsible: "eng-lead".to_string(),
            accountable: "pm".to_string(),
            consulted: vec![],
            informed: vec![],
        }
    }

    fn two_states_with_complete() -> Vec<WorkflowStateEntry> {
        vec![
            WorkflowStateEntry {
                id: "Active".to_string(),
                description: "Work underway".to_string(),
                semantic: None,
            },
            WorkflowStateEntry {
                id: "Done".to_string(),
                description: "Completed".to_string(),
                semantic: Some(WorkflowSemantic::Complete),
            },
        ]
    }

    fn valid_workflow() -> Workflow {
        Workflow {
            id: "Initiative".to_string(),
            name: "Initiative".to_string(),
            description: None,
            purpose: "Deliver features".to_string(),
            accountability: base_raci(),
            steps: vec![Step::Work(WorkStep {
                id: "Proposal".to_string(),
                depends_on: None,
            })],
            states: two_states_with_complete(),
            hooks: None,
            guidance: None,
        }
    }

    // --- 11.1: Workflow structural validator tests ---

    #[test]
    fn valid_workflow_passes_structural() {
        let errors = validate_structural(&valid_workflow());
        assert!(errors.is_empty());
    }

    #[test]
    fn workflow_kebab_id_fails() {
        let wf = Workflow {
            id: "my-initiative".to_string(),
            ..valid_workflow()
        };
        let errors = validate_structural(&wf);
        assert!(!errors.is_empty());
        assert_eq!(errors[0].path, "id");
    }

    #[test]
    fn workflow_empty_steps_fails() {
        let wf = Workflow {
            steps: vec![],
            ..valid_workflow()
        };
        let errors = validate_structural(&wf);
        assert!(errors.iter().any(|e| e.path == "steps"));
    }

    #[test]
    fn workflow_step_kebab_id_fails() {
        let wf = Workflow {
            steps: vec![Step::Work(WorkStep {
                id: "write-proposal".to_string(),
                depends_on: None,
            })],
            ..valid_workflow()
        };
        let errors = validate_structural(&wf);
        assert!(errors.iter().any(|e| e.path == "steps[0].id"));
    }

    #[test]
    fn workflow_review_step_kebab_id_fails() {
        let wf = Workflow {
            steps: vec![
                Step::Work(WorkStep {
                    id: "Proposal".to_string(),
                    depends_on: None,
                }),
                Step::Review(ReviewStep {
                    id: "legal-approval".to_string(),
                    approver: "eng-lead".to_string(),
                    on_reject: "Proposal".to_string(),
                }),
            ],
            states: vec![
                WorkflowStateEntry {
                    id: "Active".to_string(),
                    description: "desc".to_string(),
                    semantic: None,
                },
                WorkflowStateEntry {
                    id: "UnderReview".to_string(),
                    description: "desc".to_string(),
                    semantic: Some(WorkflowSemantic::Reviewing),
                },
                WorkflowStateEntry {
                    id: "Done".to_string(),
                    description: "desc".to_string(),
                    semantic: Some(WorkflowSemantic::Complete),
                },
            ],
            ..valid_workflow()
        };
        let errors = validate_structural(&wf);
        assert!(errors.iter().any(|e| e.path == "steps[1].id"));
    }

    #[test]
    fn workflow_one_state_fails() {
        let wf = Workflow {
            states: vec![WorkflowStateEntry {
                id: "Active".to_string(),
                description: "desc".to_string(),
                semantic: None,
            }],
            ..valid_workflow()
        };
        let errors = validate_structural(&wf);
        assert!(errors.iter().any(|e| e.path == "states"));
    }

    #[test]
    fn workflow_state_kebab_id_fails() {
        let wf = Workflow {
            states: vec![
                WorkflowStateEntry {
                    id: "in-progress".to_string(),
                    description: "desc".to_string(),
                    semantic: None,
                },
                WorkflowStateEntry {
                    id: "Done".to_string(),
                    description: "desc".to_string(),
                    semantic: Some(WorkflowSemantic::Complete),
                },
            ],
            ..valid_workflow()
        };
        let errors = validate_structural(&wf);
        assert!(errors.iter().any(|e| e.path == "states[0].id"));
    }

    #[test]
    fn workflow_two_states_passes() {
        let errors = validate_structural(&valid_workflow());
        assert!(errors.is_empty());
    }

    // --- 11.3: Workflow states semantic tests ---

    #[test]
    fn workflow_states_with_complete_and_non_complete_passes() {
        let errors = validate_states_semantic(&valid_workflow());
        assert!(errors.is_empty());
    }

    #[test]
    fn workflow_states_missing_complete_fails() {
        let wf = Workflow {
            states: vec![
                WorkflowStateEntry {
                    id: "Active".to_string(),
                    description: "desc".to_string(),
                    semantic: None,
                },
                WorkflowStateEntry {
                    id: "Blocked".to_string(),
                    description: "desc".to_string(),
                    semantic: Some(WorkflowSemantic::Blocked),
                },
            ],
            ..valid_workflow()
        };
        let errors = validate_states_semantic(&wf);
        assert!(errors.iter().any(|e| e.message.contains("complete")));
    }

    #[test]
    fn workflow_states_all_complete_fails() {
        let wf = Workflow {
            states: vec![
                WorkflowStateEntry {
                    id: "Done1".to_string(),
                    description: "desc".to_string(),
                    semantic: Some(WorkflowSemantic::Complete),
                },
                WorkflowStateEntry {
                    id: "Done2".to_string(),
                    description: "desc".to_string(),
                    semantic: Some(WorkflowSemantic::Complete),
                },
            ],
            ..valid_workflow()
        };
        let errors = validate_states_semantic(&wf);
        assert!(errors.iter().any(|e| e.message.contains("without semantic: complete")));
    }

    #[test]
    fn workflow_with_review_step_missing_reviewing_state_fails() {
        let wf = Workflow {
            steps: vec![
                Step::Work(WorkStep {
                    id: "Proposal".to_string(),
                    depends_on: None,
                }),
                Step::Review(ReviewStep {
                    id: "LegalApproval".to_string(),
                    approver: "eng-lead".to_string(),
                    on_reject: "Proposal".to_string(),
                }),
            ],
            states: two_states_with_complete(), // no reviewing state
            ..valid_workflow()
        };
        let errors = validate_states_semantic(&wf);
        assert!(errors.iter().any(|e| e.message.contains("reviewing")));
    }

    #[test]
    fn workflow_with_only_work_steps_no_reviewing_state_passes() {
        let errors = validate_states_semantic(&valid_workflow());
        assert!(!errors.iter().any(|e| e.message.contains("reviewing")));
    }

    // --- 11.5: Step id uniqueness tests ---

    #[test]
    fn unique_step_ids_passes() {
        let wf = Workflow {
            steps: vec![
                Step::Work(WorkStep {
                    id: "Proposal".to_string(),
                    depends_on: None,
                }),
                Step::Review(ReviewStep {
                    id: "LegalApproval".to_string(),
                    approver: "eng-lead".to_string(),
                    on_reject: "Proposal".to_string(),
                }),
            ],
            ..valid_workflow()
        };
        let errors = validate_step_id_uniqueness(&wf);
        assert!(errors.is_empty());
    }

    #[test]
    fn duplicate_work_step_id_fails() {
        let wf = Workflow {
            steps: vec![
                Step::Work(WorkStep {
                    id: "Proposal".to_string(),
                    depends_on: None,
                }),
                Step::Work(WorkStep {
                    id: "Proposal".to_string(),
                    depends_on: None,
                }),
            ],
            ..valid_workflow()
        };
        let errors = validate_step_id_uniqueness(&wf);
        assert!(!errors.is_empty());
        assert!(errors[0].message.contains("duplicate step id"));
    }

    #[test]
    fn duplicate_review_step_id_fails() {
        let wf = Workflow {
            steps: vec![
                Step::Work(WorkStep {
                    id: "Proposal".to_string(),
                    depends_on: None,
                }),
                Step::Review(ReviewStep {
                    id: "Approval".to_string(),
                    approver: "eng-lead".to_string(),
                    on_reject: "Proposal".to_string(),
                }),
                Step::Review(ReviewStep {
                    id: "Approval".to_string(),
                    approver: "pm".to_string(),
                    on_reject: "Proposal".to_string(),
                }),
            ],
            ..valid_workflow()
        };
        let errors = validate_step_id_uniqueness(&wf);
        assert!(!errors.is_empty());
        assert!(errors[0].message.contains("duplicate step id"));
    }

    #[test]
    fn duplicate_step_id_across_types_fails() {
        let wf = Workflow {
            steps: vec![
                Step::Work(WorkStep {
                    id: "Proposal".to_string(),
                    depends_on: None,
                }),
                Step::Review(ReviewStep {
                    id: "Proposal".to_string(),
                    approver: "eng-lead".to_string(),
                    on_reject: "Proposal".to_string(),
                }),
            ],
            ..valid_workflow()
        };
        let errors = validate_step_id_uniqueness(&wf);
        assert!(!errors.is_empty());
        assert!(errors[0].message.contains("duplicate step id 'Proposal'"));
    }

    // --- 11.6: State id uniqueness tests ---

    #[test]
    fn unique_state_ids_passes() {
        let errors = validate_state_id_uniqueness(&valid_workflow());
        assert!(errors.is_empty());
    }

    #[test]
    fn duplicate_state_id_fails() {
        let wf = Workflow {
            states: vec![
                WorkflowStateEntry {
                    id: "Active".to_string(),
                    description: "desc".to_string(),
                    semantic: None,
                },
                WorkflowStateEntry {
                    id: "Active".to_string(),
                    description: "desc2".to_string(),
                    semantic: Some(WorkflowSemantic::Complete),
                },
            ],
            ..valid_workflow()
        };
        let errors = validate_state_id_uniqueness(&wf);
        assert!(!errors.is_empty());
        assert!(errors[0].message.contains("duplicate state id 'Active'"));
    }

    // --- 11.7: ReviewStep on_reject ordering tests ---

    #[test]
    fn on_reject_references_earlier_step_passes() {
        let wf = Workflow {
            steps: vec![
                Step::Work(WorkStep {
                    id: "Proposal".to_string(),
                    depends_on: None,
                }),
                Step::Review(ReviewStep {
                    id: "LegalApproval".to_string(),
                    approver: "eng-lead".to_string(),
                    on_reject: "Proposal".to_string(),
                }),
            ],
            ..valid_workflow()
        };
        let errors = validate_review_step_on_reject(&wf);
        assert!(errors.is_empty());
    }

    #[test]
    fn on_reject_references_later_step_fails() {
        let wf = Workflow {
            steps: vec![
                Step::Review(ReviewStep {
                    id: "LegalApproval".to_string(),
                    approver: "eng-lead".to_string(),
                    on_reject: "Proposal".to_string(), // Proposal comes AFTER
                }),
                Step::Work(WorkStep {
                    id: "Proposal".to_string(),
                    depends_on: None,
                }),
            ],
            ..valid_workflow()
        };
        let errors = validate_review_step_on_reject(&wf);
        assert!(!errors.is_empty());
        assert!(errors[0].message.contains("earlier step"));
    }

    #[test]
    fn on_reject_references_unknown_step_fails() {
        let wf = Workflow {
            steps: vec![
                Step::Work(WorkStep {
                    id: "Proposal".to_string(),
                    depends_on: None,
                }),
                Step::Review(ReviewStep {
                    id: "LegalApproval".to_string(),
                    approver: "eng-lead".to_string(),
                    on_reject: "NonExistentStep".to_string(),
                }),
            ],
            ..valid_workflow()
        };
        let errors = validate_review_step_on_reject(&wf);
        assert!(!errors.is_empty());
        assert!(errors[0].message.contains("unknown step"));
    }

    // --- 11.9: WorkStep depends_on integrity tests ---

    #[test]
    fn depends_on_valid_step_ids_passes() {
        let wf = Workflow {
            steps: vec![
                Step::Work(WorkStep {
                    id: "Shape".to_string(),
                    depends_on: None,
                }),
                Step::Work(WorkStep {
                    id: "Proposal".to_string(),
                    depends_on: Some(vec!["Shape".to_string()]),
                }),
            ],
            ..valid_workflow()
        };
        let errors = validate_work_step_depends_on(&wf);
        assert!(errors.is_empty());
    }

    #[test]
    fn depends_on_unknown_step_fails() {
        let wf = Workflow {
            steps: vec![Step::Work(WorkStep {
                id: "Proposal".to_string(),
                depends_on: Some(vec!["UnknownStep".to_string()]),
            })],
            ..valid_workflow()
        };
        let errors = validate_work_step_depends_on(&wf);
        assert!(!errors.is_empty());
        assert!(errors[0].message.contains("unknown step 'UnknownStep'"));
    }

    // --- 11.11: Workflow RACI and HooksMap referential integrity tests ---

    #[test]
    fn workflow_valid_raci_and_hooks_passes() {
        let ctx = make_ctx();
        let mut hooks = HashMap::new();
        hooks.insert(
            "after".to_string(),
            HookInvocationValue::Single(HookInvocation::Bare("NotifySlack".to_string())),
        );
        let wf = Workflow {
            hooks: Some(hooks),
            ..valid_workflow()
        };
        let errors = validate_referential_integrity(&wf, &ctx);
        assert!(errors.is_empty());
    }

    #[test]
    fn workflow_unknown_role_in_raci_fails() {
        let mut ctx = make_ctx();
        ctx.role_ids.clear();
        let errors = validate_referential_integrity(&valid_workflow(), &ctx);
        assert!(!errors.is_empty());
        assert!(errors[0].path.contains("accountability"));
    }

    #[test]
    fn workflow_unknown_hook_in_hooks_fails() {
        let ctx = make_ctx();
        let mut hooks = HashMap::new();
        hooks.insert(
            "after".to_string(),
            HookInvocationValue::Single(HookInvocation::Bare("UnknownHook".to_string())),
        );
        let wf = Workflow {
            hooks: Some(hooks),
            ..valid_workflow()
        };
        let errors = validate_referential_integrity(&wf, &ctx);
        assert!(!errors.is_empty());
        assert!(errors[0].message.contains("unknown hook"));
    }
}
