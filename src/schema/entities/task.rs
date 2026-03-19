use std::collections::HashSet;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::schema::context::RepoContext;
use crate::schema::types::{Artifact, HooksMap, Raci, TaskSemantic, TaskStateEntry};
use crate::schema::validation::{is_camel_case, validate_hooks_map, validate_raci, ValidationError};

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct Task {
    #[schemars(regex(pattern = r"^[A-Z][A-Za-z0-9]*$"))]
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub purpose: String,
    #[schemars(length(min = 1))]
    pub instructions: Vec<String>,
    #[schemars(length(min = 1))]
    pub criteria: Vec<String>,
    pub accountability: Option<Raci>,
    pub artifact: Artifact,
    #[schemars(length(min = 2))]
    pub states: Vec<TaskStateEntry>,
    pub hooks: Option<HooksMap>,
    pub guidance: Option<String>,
}

pub fn validate(task: &Task, ctx: &RepoContext) -> Vec<ValidationError> {
    let mut errors = Vec::new();

    errors.extend(validate_structural(task));
    errors.extend(validate_states_semantic(task));
    errors.extend(validate_state_id_uniqueness(task));
    errors.extend(validate_referential_integrity(task, ctx));

    errors
}

fn validate_structural(task: &Task) -> Vec<ValidationError> {
    let mut errors = Vec::new();

    if !is_camel_case(&task.id) {
        errors.push(ValidationError {
            path: "id".to_string(),
            message: format!("id must be CamelCase, got '{}'", task.id),
        });
    }

    if task.instructions.is_empty() {
        errors.push(ValidationError {
            path: "instructions".to_string(),
            message: "instructions must have at least one item".to_string(),
        });
    }

    if task.criteria.is_empty() {
        errors.push(ValidationError {
            path: "criteria".to_string(),
            message: "criteria must have at least one item".to_string(),
        });
    }

    if task.states.len() < 2 {
        errors.push(ValidationError {
            path: "states".to_string(),
            message: format!(
                "states must have at least 2 entries, got {}",
                task.states.len()
            ),
        });
    }

    for (i, state) in task.states.iter().enumerate() {
        if !is_camel_case(&state.id) {
            errors.push(ValidationError {
                path: format!("states[{}].id", i),
                message: format!("id must be CamelCase, got '{}'", state.id),
            });
        }
    }

    errors
}

fn validate_states_semantic(task: &Task) -> Vec<ValidationError> {
    let mut errors = Vec::new();

    let has_complete = task
        .states
        .iter()
        .any(|s| s.semantic == Some(TaskSemantic::Complete));
    let has_non_complete = task
        .states
        .iter()
        .any(|s| s.semantic != Some(TaskSemantic::Complete));

    if !has_complete {
        errors.push(ValidationError {
            path: "states".to_string(),
            message: "states must include at least one entry with semantic: complete".to_string(),
        });
    }

    if !has_non_complete {
        errors.push(ValidationError {
            path: "states".to_string(),
            message: "states must include at least one entry without semantic: complete"
                .to_string(),
        });
    }

    errors
}

fn validate_state_id_uniqueness(task: &Task) -> Vec<ValidationError> {
    let mut errors = Vec::new();
    let mut seen: HashSet<&str> = HashSet::new();

    for (i, state) in task.states.iter().enumerate() {
        if !seen.insert(state.id.as_str()) {
            errors.push(ValidationError {
                path: format!("states[{}].id", i),
                message: format!("duplicate state id '{}'", state.id),
            });
        }
    }

    errors
}

fn validate_referential_integrity(task: &Task, ctx: &RepoContext) -> Vec<ValidationError> {
    let mut errors = Vec::new();

    if let Some(raci) = &task.accountability {
        errors.extend(validate_raci(raci, "accountability", ctx));
    }

    if let Some(hooks) = &task.hooks {
        errors.extend(validate_hooks_map(hooks, "hooks", ctx));
    }

    errors
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::types::{
        Artifact, HookInvocation, HookInvocationValue, Raci, TaskSemantic, TaskStateEntry,
    };
    use std::collections::HashMap;

    fn make_ctx() -> RepoContext {
        let mut ctx = RepoContext::new();
        ctx.role_ids.insert("eng-lead".to_string());
        ctx.role_ids.insert("pm".to_string());
        ctx.hook_ids.insert("NotifySlack".to_string());
        ctx
    }

    fn two_states_with_complete() -> Vec<TaskStateEntry> {
        vec![
            TaskStateEntry {
                id: "Draft".to_string(),
                description: "Being written".to_string(),
                semantic: None,
            },
            TaskStateEntry {
                id: "Done".to_string(),
                description: "Completed".to_string(),
                semantic: Some(TaskSemantic::Complete),
            },
        ]
    }

    fn valid_task() -> Task {
        Task {
            id: "Proposal".to_string(),
            name: "Proposal".to_string(),
            description: None,
            purpose: "Define the initiative scope".to_string(),
            instructions: vec!["Write the proposal".to_string()],
            criteria: vec!["Has problem statement".to_string()],
            accountability: None,
            artifact: Artifact {
                name: "proposal.md".to_string(),
                template: None,
            },
            states: two_states_with_complete(),
            hooks: None,
            guidance: None,
        }
    }

    // --- 12.1: Task structural validator tests ---

    #[test]
    fn valid_task_passes_structural() {
        let errors = validate_structural(&valid_task());
        assert!(errors.is_empty());
    }

    #[test]
    fn task_kebab_id_fails() {
        let task = Task {
            id: "write-proposal".to_string(),
            ..valid_task()
        };
        let errors = validate_structural(&task);
        assert!(!errors.is_empty());
        assert_eq!(errors[0].path, "id");
    }

    #[test]
    fn task_empty_instructions_fails() {
        let task = Task {
            instructions: vec![],
            ..valid_task()
        };
        let errors = validate_structural(&task);
        assert!(errors.iter().any(|e| e.path == "instructions"));
    }

    #[test]
    fn task_empty_criteria_fails() {
        let task = Task {
            criteria: vec![],
            ..valid_task()
        };
        let errors = validate_structural(&task);
        assert!(errors.iter().any(|e| e.path == "criteria"));
    }

    #[test]
    fn task_one_state_fails() {
        let task = Task {
            states: vec![TaskStateEntry {
                id: "Done".to_string(),
                description: "desc".to_string(),
                semantic: Some(TaskSemantic::Complete),
            }],
            ..valid_task()
        };
        let errors = validate_structural(&task);
        assert!(errors.iter().any(|e| e.path == "states"));
    }

    #[test]
    fn task_state_kebab_id_fails() {
        let task = Task {
            states: vec![
                TaskStateEntry {
                    id: "in-progress".to_string(),
                    description: "desc".to_string(),
                    semantic: None,
                },
                TaskStateEntry {
                    id: "Done".to_string(),
                    description: "desc".to_string(),
                    semantic: Some(TaskSemantic::Complete),
                },
            ],
            ..valid_task()
        };
        let errors = validate_structural(&task);
        assert!(errors.iter().any(|e| e.path == "states[0].id"));
    }

    // --- 12.3: Task states semantic tests ---

    #[test]
    fn task_states_with_complete_and_non_complete_passes() {
        let errors = validate_states_semantic(&valid_task());
        assert!(errors.is_empty());
    }

    #[test]
    fn task_states_missing_complete_fails() {
        let task = Task {
            states: vec![
                TaskStateEntry {
                    id: "Draft".to_string(),
                    description: "desc".to_string(),
                    semantic: None,
                },
                TaskStateEntry {
                    id: "Blocked".to_string(),
                    description: "desc".to_string(),
                    semantic: Some(TaskSemantic::Blocked),
                },
            ],
            ..valid_task()
        };
        let errors = validate_states_semantic(&task);
        assert!(errors.iter().any(|e| e.message.contains("complete")));
    }

    #[test]
    fn task_states_all_complete_fails() {
        let task = Task {
            states: vec![
                TaskStateEntry {
                    id: "Done1".to_string(),
                    description: "desc".to_string(),
                    semantic: Some(TaskSemantic::Complete),
                },
                TaskStateEntry {
                    id: "Done2".to_string(),
                    description: "desc".to_string(),
                    semantic: Some(TaskSemantic::Complete),
                },
            ],
            ..valid_task()
        };
        let errors = validate_states_semantic(&task);
        assert!(errors.iter().any(|e| e.message.contains("without")));
    }

    #[test]
    fn task_reviewing_semantic_not_available_at_type_level() {
        // TaskSemantic has no Reviewing variant — the type system enforces this.
        // This test documents the constraint.
        let _blocked: TaskSemantic = TaskSemantic::Blocked;
        let _complete: TaskSemantic = TaskSemantic::Complete;
        let _failed: TaskSemantic = TaskSemantic::Failed;
        // TaskSemantic::Reviewing would be a compile error
    }

    // --- 12.4: Task state id uniqueness tests ---

    #[test]
    fn unique_task_state_ids_passes() {
        let errors = validate_state_id_uniqueness(&valid_task());
        assert!(errors.is_empty());
    }

    #[test]
    fn duplicate_task_state_id_fails() {
        let task = Task {
            states: vec![
                TaskStateEntry {
                    id: "Draft".to_string(),
                    description: "desc".to_string(),
                    semantic: None,
                },
                TaskStateEntry {
                    id: "Draft".to_string(),
                    description: "desc2".to_string(),
                    semantic: Some(TaskSemantic::Complete),
                },
            ],
            ..valid_task()
        };
        let errors = validate_state_id_uniqueness(&task);
        assert!(!errors.is_empty());
        assert!(errors[0].message.contains("duplicate state id 'Draft'"));
    }

    // --- 12.5: Task RACI and HooksMap referential integrity tests ---

    #[test]
    fn task_no_raci_passes_referential_integrity() {
        let ctx = make_ctx();
        let errors = validate_referential_integrity(&valid_task(), &ctx);
        assert!(errors.is_empty());
    }

    #[test]
    fn task_valid_raci_passes() {
        let ctx = make_ctx();
        let task = Task {
            accountability: Some(Raci {
                responsible: "eng-lead".to_string(),
                accountable: "pm".to_string(),
                consulted: vec![],
                informed: vec![],
            }),
            ..valid_task()
        };
        let errors = validate_referential_integrity(&task, &ctx);
        assert!(errors.is_empty());
    }

    #[test]
    fn task_unknown_role_in_raci_fails() {
        let ctx = make_ctx();
        let task = Task {
            accountability: Some(Raci {
                responsible: "unknown-role".to_string(),
                accountable: "pm".to_string(),
                consulted: vec![],
                informed: vec![],
            }),
            ..valid_task()
        };
        let errors = validate_referential_integrity(&task, &ctx);
        assert!(!errors.is_empty());
        assert!(errors[0].path.contains("accountability.responsible"));
    }

    #[test]
    fn task_unknown_hook_in_hooks_fails() {
        let ctx = make_ctx();
        let mut hooks = HashMap::new();
        hooks.insert(
            "after".to_string(),
            HookInvocationValue::Single(HookInvocation::Bare("UnknownHook".to_string())),
        );
        let task = Task {
            hooks: Some(hooks),
            ..valid_task()
        };
        let errors = validate_referential_integrity(&task, &ctx);
        assert!(!errors.is_empty());
        assert!(errors[0].message.contains("unknown hook"));
    }

    #[test]
    fn task_valid_hook_passes() {
        let ctx = make_ctx();
        let mut hooks = HashMap::new();
        hooks.insert(
            "after".to_string(),
            HookInvocationValue::Single(HookInvocation::Bare("NotifySlack".to_string())),
        );
        let task = Task {
            hooks: Some(hooks),
            ..valid_task()
        };
        let errors = validate_referential_integrity(&task, &ctx);
        assert!(errors.is_empty());
    }
}
