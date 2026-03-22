//! [`Workflow`] and [`SharedWorkflow`] entities — step sequences with state machines.

use std::collections::HashSet;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::schema::{
    entities::{
        relay::{self, Relay},
        task::{self, Task},
    },
    ids::WorkflowId,
    store::EntityStore,
    types::{Extensions, HooksMap, Raci, WorkflowSemantic, WorkflowStateEntry},
    validation::{
        is_camel_case, validate_extensions, validate_hooks_map, validate_raci, ValidationError,
    },
};

// --- HasId ---

pub trait HasId {
    fn id(&self) -> &str;
}

// --- ReviewStep (moved here from types.rs; step types live with workflow to avoid circular deps) ---

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct ReviewStep {
    #[schemars(regex(pattern = r"^[A-Z][A-Za-z0-9]*$"))]
    pub id: String,
    pub approver: String,
    pub on_reject: String,
}

// --- WorkStep<S> ---

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct WorkStep<S> {
    pub depends_on: Option<Vec<String>>,
    pub definition: S,
}

// --- Step<S> ---

#[allow(clippy::large_enum_variant)]
#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum Step<S> {
    Work(WorkStep<S>),
    Review(ReviewStep),
}

impl<S: HasId> Step<S> {
    pub fn id(&self) -> &str {
        match self {
            Self::Work(ws) => ws.definition.id(),
            Self::Review(rs) => &rs.id,
        }
    }
}

// --- WorkStepDefinition ---

/// Embedded definition inside a `WorkStep`. Discriminated by required fields:
/// - Task   → has `artifact`
/// - Relay  → has `delegates_to`
/// - Workflow → has `steps`
#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum WorkStepDefinition {
    Task(Task),
    Relay(Relay),
    Workflow(Box<Workflow>),
}

impl HasId for WorkStepDefinition {
    fn id(&self) -> &str {
        match self {
            Self::Task(t) => t.id.as_ref(),
            Self::Relay(r) => r.id.as_ref(),
            Self::Workflow(wf) => wf.id.as_ref(),
        }
    }
}

// --- SharedWorkStepDefinition ---

/// Embedded definition inside a `SharedWorkStep`. No Relay variant (shared workflows cannot relay).
#[allow(clippy::large_enum_variant)]
#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum SharedWorkStepDefinition {
    Task(Task),
    SharedWorkflow(Box<SharedWorkflow>),
}

impl HasId for SharedWorkStepDefinition {
    fn id(&self) -> &str {
        match self {
            Self::Task(t) => t.id.as_ref(),
            Self::SharedWorkflow(wf) => wf.id.as_ref(),
        }
    }
}

// --- Type aliases for shared step types ---

pub type SharedWorkStep = WorkStep<SharedWorkStepDefinition>;
pub type SharedStep = Step<SharedWorkStepDefinition>;

// --- WorkflowDef<S> ---

#[derive(Serialize, Deserialize, JsonSchema)]
#[schemars(deny_unknown_fields)]
pub struct WorkflowDef<S>
where
    S: JsonSchema + Serialize,
{
    pub id: WorkflowId,
    pub name: String,
    pub description: Option<String>,
    pub purpose: String,
    pub accountability: Raci,
    #[schemars(length(min = 1))]
    pub steps: Vec<Step<S>>,
    #[schemars(length(min = 2))]
    pub states: Vec<WorkflowStateEntry>,
    pub hooks: Option<HooksMap>,
    pub guidance: Option<String>,
    #[serde(flatten)]
    pub extensions: Extensions,
}

/// Workflow with embedded step definitions (Task, Relay, inline Workflow).
pub type Workflow = WorkflowDef<WorkStepDefinition>;

/// `SharedWorkflow`: like Workflow but steps cannot embed Relay.
pub type SharedWorkflow = WorkflowDef<SharedWorkStepDefinition>;

// --- Validators ---

fn prefix_errors(errors: Vec<ValidationError>, prefix: &str) -> Vec<ValidationError> {
    errors
        .into_iter()
        .map(|e| ValidationError {
            path: format!("{}.{}", prefix, e.path),
            message: e.message,
        })
        .collect()
}

pub fn validate(workflow: &Workflow, ctx: &EntityStore) -> Vec<ValidationError> {
    let structural = validate_structure_tree(workflow, ctx);
    if !structural.is_empty() {
        return structural;
    }
    validate_semantic_tree(workflow, ctx)
}

pub fn validate_shared(
    shared_workflow: &SharedWorkflow,
    ctx: &EntityStore,
) -> Vec<ValidationError> {
    let structural = validate_shared_structure_tree(shared_workflow, ctx);
    if !structural.is_empty() {
        return structural;
    }
    validate_shared_semantic_tree(shared_workflow, ctx)
}

fn validate_structure_tree(workflow: &Workflow, ctx: &EntityStore) -> Vec<ValidationError> {
    let mut errors = Vec::new();

    errors.extend(validate_structural_fields(
        &workflow.id,
        &workflow.steps,
        &workflow.states,
    ));
    errors.extend(validate_extensions(&workflow.extensions, "extensions"));

    for (i, step) in workflow.steps.iter().enumerate() {
        if let Step::Work(ws) = step {
            let prefix = format!("steps[{i}].definition");
            let child_errors = match &ws.definition {
                WorkStepDefinition::Task(t) => task::validate(t, ctx),
                WorkStepDefinition::Relay(r) => relay::validate(r, ctx),
                WorkStepDefinition::Workflow(wf) => validate(wf, ctx),
            };
            errors.extend(prefix_errors(child_errors, &prefix));
        }
    }

    errors
}

fn validate_shared_structure_tree(
    workflow: &SharedWorkflow,
    ctx: &EntityStore,
) -> Vec<ValidationError> {
    let mut errors = Vec::new();

    errors.extend(validate_structural_fields(
        &workflow.id,
        &workflow.steps,
        &workflow.states,
    ));
    errors.extend(validate_extensions(&workflow.extensions, "extensions"));

    for (i, step) in workflow.steps.iter().enumerate() {
        if let Step::Work(ws) = step {
            let prefix = format!("steps[{i}].definition");
            let child_errors = match &ws.definition {
                SharedWorkStepDefinition::Task(t) => task::validate(t, ctx),
                SharedWorkStepDefinition::SharedWorkflow(wf) => validate_shared(wf, ctx),
            };
            errors.extend(prefix_errors(child_errors, &prefix));
        }
    }

    errors
}

fn validate_structural_fields<S>(
    id: &str,
    steps: &[S],
    states: &[WorkflowStateEntry],
) -> Vec<ValidationError> {
    let mut errors = Vec::new();

    if !is_camel_case(id) {
        errors.push(ValidationError {
            path: "id".to_string(),
            message: format!("id must be CamelCase, got '{id}'"),
        });
    }

    if steps.is_empty() {
        errors.push(ValidationError {
            path: "steps".to_string(),
            message: "steps must have at least one item".to_string(),
        });
    }

    if states.len() < 2 {
        errors.push(ValidationError {
            path: "states".to_string(),
            message: format!("states must have at least 2 entries, got {}", states.len()),
        });
    }

    for (i, state) in states.iter().enumerate() {
        if !is_camel_case(&state.id) {
            errors.push(ValidationError {
                path: format!("states[{i}].id"),
                message: format!("id must be CamelCase, got '{}'", state.id),
            });
        }
    }

    errors
}


fn validate_semantic_tree(workflow: &Workflow, ctx: &EntityStore) -> Vec<ValidationError> {
    let mut errors = Vec::new();
    errors.extend(validate_states_semantic_workflow(workflow));
    errors.extend(validate_step_id_uniqueness_workflow(workflow));
    errors.extend(validate_state_id_uniqueness_workflow(workflow));
    errors.extend(validate_review_step_on_reject(workflow));
    errors.extend(validate_work_step_depends_on(workflow));
    errors.extend(validate_referential_integrity(workflow, ctx));
    errors
}

fn validate_shared_semantic_tree(
    workflow: &SharedWorkflow,
    ctx: &EntityStore,
) -> Vec<ValidationError> {
    let mut errors = Vec::new();
    errors.extend(validate_states_semantic_shared(workflow));
    errors.extend(validate_step_id_uniqueness_shared(workflow));
    errors.extend(validate_state_id_uniqueness_workflow_states(
        &workflow.states,
    ));
    errors.extend(validate_review_step_on_reject_shared(workflow));
    errors.extend(validate_work_step_depends_on_shared(workflow));
    errors.extend(validate_raci(
        &workflow.accountability,
        "accountability",
        ctx,
    ));
    if let Some(hooks) = &workflow.hooks {
        errors.extend(validate_hooks_map(hooks, "hooks", ctx));
    }
    errors
}

fn validate_states_semantic_workflow(workflow: &Workflow) -> Vec<ValidationError> {
    let mut errors = validate_states_semantic_core(&workflow.states);

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

fn validate_states_semantic_shared(workflow: &SharedWorkflow) -> Vec<ValidationError> {
    let mut errors = validate_states_semantic_core(&workflow.states);

    let has_review_step = workflow
        .steps
        .iter()
        .any(|s| matches!(s, Step::Review(_)));
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

fn validate_states_semantic_core(states: &[WorkflowStateEntry]) -> Vec<ValidationError> {
    let mut errors = Vec::new();

    let has_complete = states
        .iter()
        .any(|s| s.semantic == Some(WorkflowSemantic::Complete));
    let has_non_complete = states
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
            message: "states must include at least one entry without semantic: complete"
                .to_string(),
        });
    }

    errors
}

fn validate_step_id_uniqueness_workflow(workflow: &Workflow) -> Vec<ValidationError> {
    validate_step_id_uniqueness_core(workflow.steps.iter().map(Step::id))
}

fn validate_step_id_uniqueness_shared(workflow: &SharedWorkflow) -> Vec<ValidationError> {
    validate_step_id_uniqueness_core(workflow.steps.iter().map(Step::id))
}

fn validate_step_id_uniqueness_core<'a>(
    ids: impl Iterator<Item = &'a str>,
) -> Vec<ValidationError> {
    let mut errors = Vec::new();
    let mut seen = HashSet::new();
    for (i, id) in ids.enumerate() {
        if !seen.insert(id) {
            errors.push(ValidationError {
                path: format!("steps[{i}]"),
                message: format!("duplicate step id '{id}'"),
            });
        }
    }
    errors
}

fn validate_state_id_uniqueness_workflow(workflow: &Workflow) -> Vec<ValidationError> {
    validate_state_id_uniqueness_workflow_states(&workflow.states)
}

fn validate_state_id_uniqueness_workflow_states(
    states: &[WorkflowStateEntry],
) -> Vec<ValidationError> {
    let mut errors = Vec::new();
    let mut seen: HashSet<&str> = HashSet::new();
    for (i, state) in states.iter().enumerate() {
        if !seen.insert(state.id.as_str()) {
            errors.push(ValidationError {
                path: format!("states[{i}].id"),
                message: format!("duplicate state id '{}'", state.id),
            });
        }
    }
    errors
}

fn validate_review_step_on_reject(workflow: &Workflow) -> Vec<ValidationError> {
    validate_review_step_on_reject_core(
        &workflow.steps,
        |s| matches!(s, Step::Review(_)),
        |s| {
            if let Step::Review(rs) = s {
                Some((&rs.id, &rs.on_reject))
            } else {
                None
            }
        },
        |s| s.id(),
    )
}

fn validate_review_step_on_reject_shared(workflow: &SharedWorkflow) -> Vec<ValidationError> {
    validate_review_step_on_reject_core(
        &workflow.steps,
        |s| matches!(s, Step::Review(_)),
        |s| {
            if let Step::Review(rs) = s {
                Some((&rs.id, &rs.on_reject))
            } else {
                None
            }
        },
        |s| s.id(),
    )
}

fn validate_review_step_on_reject_core<S>(
    steps: &[S],
    is_review: impl Fn(&S) -> bool,
    review_fields: impl Fn(&S) -> Option<(&str, &str)>,
    step_id: impl Fn(&S) -> &str,
) -> Vec<ValidationError> {
    let mut errors = Vec::new();
    for (i, step) in steps.iter().enumerate() {
        if is_review(step) {
            if let Some((_id, on_reject)) = review_fields(step) {
                let ids_before: HashSet<&str> = steps[..i].iter().map(&step_id).collect();
                if !ids_before.contains(on_reject) {
                    let exists_anywhere = steps.iter().any(|s| step_id(s) == on_reject);
                    let message = if exists_anywhere {
                        format!(
                            "on_reject '{on_reject}' must reference an earlier step, not a later one"
                        )
                    } else {
                        format!("on_reject '{on_reject}' references unknown step")
                    };
                    errors.push(ValidationError {
                        path: format!("steps[{i}].on_reject"),
                        message,
                    });
                }
            }
        }
    }
    errors
}

fn validate_work_step_depends_on(workflow: &Workflow) -> Vec<ValidationError> {
    let all_ids: HashSet<&str> = workflow.steps.iter().map(Step::id).collect();
    validate_depends_on_core(&workflow.steps, &all_ids, |s| {
        if let Step::Work(ws) = s {
            ws.depends_on.as_deref()
        } else {
            None
        }
    })
}

fn validate_work_step_depends_on_shared(workflow: &SharedWorkflow) -> Vec<ValidationError> {
    let all_ids: HashSet<&str> = workflow.steps.iter().map(Step::id).collect();
    validate_depends_on_core(&workflow.steps, &all_ids, |s| {
        if let Step::Work(ws) = s {
            ws.depends_on.as_deref()
        } else {
            None
        }
    })
}

fn validate_depends_on_core<S>(
    steps: &[S],
    all_ids: &HashSet<&str>,
    get_depends_on: impl Fn(&S) -> Option<&[String]>,
) -> Vec<ValidationError> {
    let mut errors = Vec::new();
    for (i, step) in steps.iter().enumerate() {
        if let Some(deps) = get_depends_on(step) {
            for dep in deps {
                if !all_ids.contains(dep.as_str()) {
                    errors.push(ValidationError {
                        path: format!("steps[{i}].depends_on"),
                        message: format!("depends_on references unknown step '{dep}'"),
                    });
                }
            }
        }
    }
    errors
}

fn validate_referential_integrity(workflow: &Workflow, ctx: &EntityStore) -> Vec<ValidationError> {
    let mut errors = Vec::new();
    errors.extend(validate_raci(
        &workflow.accountability,
        "accountability",
        ctx,
    ));
    if let Some(hooks) = &workflow.hooks {
        errors.extend(validate_hooks_map(hooks, "hooks", ctx));
    }
    errors
}

// --- Tests ---

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;
    use crate::schema::{
        store::EntityStore,
        types::{
            Artifact, HookInvocation, HookInvocationValue, Raci, RelayStateSemantic, StateMapEntry,
            TaskSemantic, TaskStateEntry, WorkflowSemantic, WorkflowStateEntry,
        },
    };

    // --- Test helpers ---

    fn make_ctx() -> EntityStore {
        use crate::schema::{
            entities::{hook::Hook, role::Role},
            types::Extensions,
        };
        let mut ctx = EntityStore::new();
        ctx.roles.insert(
            "eng-lead".to_string(),
            Role {
                id: "eng-lead".into(),
                name: "Engineering Lead".to_string(),
                purpose: "test".to_string(),
                traits: None,
                extensions: Extensions::default(),
            },
        );
        ctx.roles.insert(
            "pm".to_string(),
            Role {
                id: "pm".into(),
                name: "Product Manager".to_string(),
                purpose: "test".to_string(),
                traits: None,
                extensions: Extensions::default(),
            },
        );
        ctx.hooks.insert(
            "NotifySlack".to_string(),
            Hook {
                id: "NotifySlack".into(),
                name: "Notify Slack".to_string(),
                description: "test".to_string(),
                instructions: vec!["send message".to_string()],
                inputs: None,
                extensions: Extensions::default(),
            },
        );
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

    fn task_two_states() -> Vec<TaskStateEntry> {
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

    fn valid_task_entity(id: &str) -> Task {
        Task {
            id: id.into(),
            name: format!("Task {}", id),
            description: None,
            purpose: "Some purpose".to_string(),
            instructions: vec!["Do the thing".to_string()],
            criteria: vec!["Thing was done".to_string()],
            accountability: None,
            artifact: Artifact {
                name: "output.md".to_string(),
                template: None,
            },
            states: task_two_states(),
            hooks: None,
            guidance: None,
            extensions: Extensions::default(),
        }
    }

    fn valid_relay_entity(id: &str, delegates_to: &str, ctx_state_ids: &[&str]) -> Relay {
        let mut state_map = HashMap::new();
        if !ctx_state_ids.is_empty() {
            state_map.insert(
                ctx_state_ids[0].to_string(),
                StateMapEntry {
                    maps_to: "Active".to_string(),
                    semantic: None,
                },
            );
        }
        state_map.insert(
            "Done".to_string(),
            StateMapEntry {
                maps_to: "Complete".to_string(),
                semantic: Some(RelayStateSemantic::Complete),
            },
        );
        Relay {
            id: id.into(),
            name: format!("Relay {}", id),
            description: None,
            purpose: "Delegate work".to_string(),
            accountability: None,
            delegates_to: delegates_to.to_string(),
            briefing: None,
            debriefing: None,
            state_map,
            hooks: None,
            guidance: None,
            extensions: Extensions::default(),
        }
    }

    fn task_step(id: &str) -> Step<WorkStepDefinition> {
        Step::Work(WorkStep {
            depends_on: None,
            definition: WorkStepDefinition::Task(valid_task_entity(id)),
        })
    }

    fn task_step_with_deps(id: &str, deps: Vec<String>) -> Step<WorkStepDefinition> {
        Step::Work(WorkStep {
            depends_on: Some(deps),
            definition: WorkStepDefinition::Task(valid_task_entity(id)),
        })
    }

    fn review_step_item(id: &str, approver: &str, on_reject: &str) -> Step<WorkStepDefinition> {
        Step::Review(ReviewStep {
            id: id.to_string(),
            approver: approver.to_string(),
            on_reject: on_reject.to_string(),
        })
    }

    fn valid_workflow() -> Workflow {
        WorkflowDef {
            id: "Initiative".into(),
            name: "Initiative".to_string(),
            description: None,
            purpose: "Deliver features".to_string(),
            accountability: base_raci(),
            steps: vec![task_step("Proposal")],
            states: two_states_with_complete(),
            hooks: None,
            guidance: None,
            extensions: Extensions::default(),
        }
    }

    // --- 9.1: SharedWorkflow step type constraint tests ---

    #[test]
    fn shared_workflow_relay_step_excluded_at_type_level() {
        // SharedWorkStepDefinition has no Relay variant — enforced by the type system.
        // This test documents the constraint via exhaustive match (compilation proves it).
        let def = SharedWorkStepDefinition::Task(valid_task_entity("Scope"));
        match def {
            SharedWorkStepDefinition::Task(_) => {}
            SharedWorkStepDefinition::SharedWorkflow(_) => {} // No Relay variant — compiler would reject it
        }
    }

    #[test]
    fn shared_workflow_with_task_step_validates_successfully() {
        let sw: SharedWorkflow = WorkflowDef {
            id: "SharedOnboarding".into(),
            name: "Shared Onboarding".to_string(),
            description: None,
            purpose: "Reusable onboarding flow".to_string(),
            accountability: base_raci(),
            steps: vec![shared_task_step("Orientation")],
            states: two_states_with_complete(),
            hooks: None,
            guidance: None,
            extensions: Extensions::default(),
        };
        let errors = validate_shared(&sw, &make_ctx());
        assert!(
            errors.is_empty(),
            "Got errors: {:?}",
            errors.iter().map(|e| &e.message).collect::<Vec<_>>()
        );
    }

    #[test]
    fn shared_workflow_with_review_step_validates_successfully() {
        let sw: SharedWorkflow = WorkflowDef {
            id: "SharedReview".into(),
            name: "Shared Review".to_string(),
            description: None,
            purpose: "Reusable review flow".to_string(),
            accountability: base_raci(),
            steps: vec![
                shared_task_step("Scope"),
                shared_review_step_item("Approval", "pm", "Scope"),
            ],
            states: vec![
                WorkflowStateEntry {
                    id: "Active".to_string(),
                    description: "Work underway".to_string(),
                    semantic: None,
                },
                WorkflowStateEntry {
                    id: "Reviewing".to_string(),
                    description: "Under review".to_string(),
                    semantic: Some(WorkflowSemantic::Reviewing),
                },
                WorkflowStateEntry {
                    id: "Done".to_string(),
                    description: "Completed".to_string(),
                    semantic: Some(WorkflowSemantic::Complete),
                },
            ],
            hooks: None,
            guidance: None,
            extensions: Extensions::default(),
        };
        let errors = validate_shared(&sw, &make_ctx());
        assert!(
            errors.is_empty(),
            "Got errors: {:?}",
            errors.iter().map(|e| &e.message).collect::<Vec<_>>()
        );
    }

    // --- 8.2: Workflow extensions validation tests ---

    #[test]
    fn workflow_x_prefixed_extension_passes() {
        let mut map = HashMap::new();
        map.insert("x-epic".to_string(), serde_json::json!("ENG-100"));
        let wf = WorkflowDef {
            extensions: Extensions(map),
            ..valid_workflow()
        };
        let errors = validate_structure_tree(&wf, &make_ctx());
        assert!(errors.is_empty());
    }

    #[test]
    fn workflow_non_x_extension_key_fails() {
        let mut map = HashMap::new();
        map.insert("epic".to_string(), serde_json::json!("ENG-100"));
        let wf = WorkflowDef {
            extensions: Extensions(map),
            ..valid_workflow()
        };
        let errors = validate_structure_tree(&wf, &make_ctx());
        assert!(!errors.is_empty());
        assert!(errors[0].path.contains("extensions"));
        assert!(errors[0].message.contains("x-"));
    }

    // --- Task 4.1: WorkStepDefinition discrimination tests ---

    #[test]
    fn work_step_definition_id_from_task() {
        let ws = WorkStepDefinition::Task(valid_task_entity("Proposal"));
        assert_eq!(ws.id(), "Proposal");
    }

    #[test]
    fn work_step_definition_id_from_relay() {
        let relay = valid_relay_entity("LegalSignoff", "LegalReview", &["Active"]);
        let ws = WorkStepDefinition::Relay(relay);
        assert_eq!(ws.id(), "LegalSignoff");
    }

    #[test]
    fn work_step_definition_id_from_inline_workflow() {
        let inner = WorkflowDef {
            id: "Kickoff".into(),
            name: "Kickoff".to_string(),
            description: None,
            purpose: "Kick off the project".to_string(),
            accountability: base_raci(),
            steps: vec![task_step("Setup")],
            states: two_states_with_complete(),
            hooks: None,
            guidance: None,
            extensions: Extensions::default(),
        };
        let ws = WorkStepDefinition::Workflow(Box::new(inner));
        assert_eq!(ws.id(), "Kickoff");
    }

    #[test]
    fn step_id_for_work_step_returns_definition_id() {
        let step = task_step("Proposal");
        assert_eq!(step.id(), "Proposal");
    }

    #[test]
    fn step_id_for_review_step_returns_review_id() {
        let step = review_step_item("LegalApproval", "eng-lead", "Proposal");
        assert_eq!(step.id(), "LegalApproval");
    }

    // --- Task 4.3: SharedWorkStepDefinition, SharedWorkStep, SharedStep tests ---

    fn shared_task_step(id: &str) -> Step<SharedWorkStepDefinition> {
        Step::Work(WorkStep {
            depends_on: None,
            definition: SharedWorkStepDefinition::Task(valid_task_entity(id)),
        })
    }

    fn shared_review_step_item(id: &str, approver: &str, on_reject: &str) -> Step<SharedWorkStepDefinition> {
        Step::Review(ReviewStep {
            id: id.to_string(),
            approver: approver.to_string(),
            on_reject: on_reject.to_string(),
        })
    }

    #[test]
    fn shared_work_step_definition_id_from_task() {
        let def = SharedWorkStepDefinition::Task(valid_task_entity("Scope"));
        assert_eq!(def.id(), "Scope");
    }

    #[test]
    fn shared_work_step_definition_id_from_shared_workflow() {
        let inner: SharedWorkflow = WorkflowDef {
            id: "ReviewProcess".into(),
            name: "Review".to_string(),
            description: None,
            purpose: "Review the work".to_string(),
            accountability: base_raci(),
            steps: vec![shared_task_step("Review")],
            states: two_states_with_complete(),
            hooks: None,
            guidance: None,
            extensions: Extensions::default(),
        };
        let def = SharedWorkStepDefinition::SharedWorkflow(Box::new(inner));
        assert_eq!(def.id(), "ReviewProcess");
    }

    #[test]
    fn shared_step_id_for_work_step() {
        let step = shared_task_step("Scope");
        assert_eq!(step.id(), "Scope");
    }

    #[test]
    fn shared_step_id_for_review_step() {
        let step = shared_review_step_item("Approval", "pm", "Scope");
        assert_eq!(step.id(), "Approval");
    }

    // --- Task 5.1: WorkflowDef<Step> and WorkflowDef<SharedStep> construction ---

    #[test]
    fn workflow_def_with_task_step_constructs() {
        let wf = valid_workflow();
        assert_eq!(wf.id, "Initiative");
        assert_eq!(wf.steps.len(), 1);
        assert_eq!(wf.steps[0].id(), "Proposal");
    }

    #[test]
    fn shared_workflow_def_constructs() {
        let wf: SharedWorkflow = WorkflowDef {
            id: "SharedReview".into(),
            name: "Shared Review".to_string(),
            description: None,
            purpose: "Reusable review flow".to_string(),
            accountability: base_raci(),
            steps: vec![shared_task_step("Scope")],
            states: two_states_with_complete(),
            hooks: None,
            guidance: None,
            extensions: Extensions::default(),
        };
        assert_eq!(wf.id, "SharedReview");
        assert_eq!(wf.steps[0].id(), "Scope");
    }

    // --- Task 5.3: Updated existing workflow validator tests ---

    #[test]
    fn valid_workflow_passes_structural() {
        let errors = validate_structure_tree(&valid_workflow(), &make_ctx());
        assert!(
            errors.is_empty(),
            "Got errors: {:?}",
            errors.iter().map(|e| &e.message).collect::<Vec<_>>()
        );
    }

    #[test]
    fn workflow_kebab_id_fails() {
        let wf = WorkflowDef {
            id: "my-initiative".into(),
            ..valid_workflow()
        };
        let errors = validate_structure_tree(&wf, &make_ctx());
        assert!(errors.iter().any(|e| e.path == "id"));
    }

    #[test]
    fn workflow_empty_steps_fails() {
        let wf = WorkflowDef {
            steps: vec![],
            ..valid_workflow()
        };
        let errors = validate_structure_tree(&wf, &make_ctx());
        assert!(errors.iter().any(|e| e.path == "steps"));
    }

    #[test]
    fn workflow_one_state_fails() {
        let wf = WorkflowDef {
            states: vec![WorkflowStateEntry {
                id: "Active".to_string(),
                description: "desc".to_string(),
                semantic: None,
            }],
            ..valid_workflow()
        };
        let errors = validate_structure_tree(&wf, &make_ctx());
        assert!(errors.iter().any(|e| e.path == "states"));
    }

    #[test]
    fn workflow_state_kebab_id_fails() {
        let wf = WorkflowDef {
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
        let errors = validate_structure_tree(&wf, &make_ctx());
        assert!(errors.iter().any(|e| e.path == "states[0].id"));
    }

    #[test]
    fn workflow_states_missing_complete_fails() {
        let wf = WorkflowDef {
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
        let errors = validate_semantic_tree(&wf, &make_ctx());
        assert!(errors.iter().any(|e| e.message.contains("complete")));
    }

    #[test]
    fn workflow_states_all_complete_fails() {
        let wf = WorkflowDef {
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
        let errors = validate_semantic_tree(&wf, &make_ctx());
        assert!(errors.iter().any(|e| e.message.contains("without")));
    }

    #[test]
    fn workflow_with_review_step_missing_reviewing_state_fails() {
        let wf = WorkflowDef {
            steps: vec![
                task_step("Proposal"),
                review_step_item("LegalApproval", "eng-lead", "Proposal"),
            ],
            states: two_states_with_complete(), // no reviewing state
            ..valid_workflow()
        };
        let errors = validate_semantic_tree(&wf, &make_ctx());
        assert!(errors.iter().any(|e| e.message.contains("reviewing")));
    }

    #[test]
    fn workflow_with_only_task_steps_no_reviewing_state_passes() {
        let errors = validate_semantic_tree(&valid_workflow(), &make_ctx());
        assert!(!errors.iter().any(|e| e.message.contains("reviewing")));
    }

    #[test]
    fn unique_step_ids_passes() {
        let wf = WorkflowDef {
            steps: vec![
                task_step("Proposal"),
                review_step_item("LegalApproval", "eng-lead", "Proposal"),
            ],
            ..valid_workflow()
        };
        let errors = validate_step_id_uniqueness_workflow(&wf);
        assert!(errors.is_empty());
    }

    #[test]
    fn duplicate_step_id_fails() {
        let wf = WorkflowDef {
            steps: vec![task_step("Proposal"), task_step("Proposal")],
            ..valid_workflow()
        };
        let errors = validate_step_id_uniqueness_workflow(&wf);
        assert!(!errors.is_empty());
        assert!(errors[0].message.contains("duplicate step id 'Proposal'"));
    }

    #[test]
    fn unique_state_ids_passes() {
        let errors = validate_state_id_uniqueness_workflow(&valid_workflow());
        assert!(errors.is_empty());
    }

    #[test]
    fn duplicate_state_id_fails() {
        let wf = WorkflowDef {
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
        let errors = validate_state_id_uniqueness_workflow(&wf);
        assert!(!errors.is_empty());
        assert!(errors[0].message.contains("duplicate state id 'Active'"));
    }

    #[test]
    fn on_reject_references_earlier_step_passes() {
        let wf = WorkflowDef {
            steps: vec![
                task_step("Proposal"),
                review_step_item("LegalApproval", "eng-lead", "Proposal"),
            ],
            ..valid_workflow()
        };
        let errors = validate_review_step_on_reject(&wf);
        assert!(errors.is_empty());
    }

    #[test]
    fn on_reject_references_later_step_fails() {
        let wf = WorkflowDef {
            steps: vec![
                review_step_item("LegalApproval", "eng-lead", "Proposal"), // Proposal comes after
                task_step("Proposal"),
            ],
            ..valid_workflow()
        };
        let errors = validate_review_step_on_reject(&wf);
        assert!(!errors.is_empty());
        assert!(errors[0].message.contains("earlier step"));
    }

    #[test]
    fn on_reject_references_unknown_step_fails() {
        let wf = WorkflowDef {
            steps: vec![
                task_step("Proposal"),
                review_step_item("LegalApproval", "eng-lead", "NonExistent"),
            ],
            ..valid_workflow()
        };
        let errors = validate_review_step_on_reject(&wf);
        assert!(!errors.is_empty());
        assert!(errors[0].message.contains("unknown step"));
    }

    #[test]
    fn depends_on_valid_step_id_passes() {
        let wf = WorkflowDef {
            steps: vec![
                task_step("Shape"),
                task_step_with_deps("Proposal", vec!["Shape".to_string()]),
            ],
            ..valid_workflow()
        };
        let errors = validate_work_step_depends_on(&wf);
        assert!(errors.is_empty());
    }

    #[test]
    fn depends_on_unknown_step_fails() {
        let wf = WorkflowDef {
            steps: vec![task_step_with_deps(
                "Proposal",
                vec!["UnknownStep".to_string()],
            )],
            ..valid_workflow()
        };
        let errors = validate_work_step_depends_on(&wf);
        assert!(!errors.is_empty());
        assert!(errors[0].message.contains("unknown step 'UnknownStep'"));
    }

    #[test]
    fn workflow_valid_raci_and_hooks_passes() {
        let ctx = make_ctx();
        let mut hooks = HashMap::new();
        hooks.insert(
            "after".to_string(),
            HookInvocationValue::Single(HookInvocation::Bare("NotifySlack".to_string())),
        );
        let wf = WorkflowDef {
            hooks: Some(hooks),
            ..valid_workflow()
        };
        let errors = validate_referential_integrity(&wf, &ctx);
        assert!(errors.is_empty());
    }

    #[test]
    fn workflow_unknown_role_in_raci_fails() {
        let mut ctx = make_ctx();
        ctx.roles.clear();
        let errors = validate_referential_integrity(&valid_workflow(), &ctx);
        assert!(!errors.is_empty());
        assert!(errors[0].path.contains("accountability"));
    }

    #[test]
    fn workflow_unknown_hook_fails() {
        let ctx = make_ctx();
        let mut hooks = HashMap::new();
        hooks.insert(
            "after".to_string(),
            HookInvocationValue::Single(HookInvocation::Bare("UnknownHook".to_string())),
        );
        let wf = WorkflowDef {
            hooks: Some(hooks),
            ..valid_workflow()
        };
        let errors = validate_referential_integrity(&wf, &ctx);
        assert!(!errors.is_empty());
        assert!(errors[0].message.contains("unknown hook"));
    }

    // --- Task 7.3: Validator composition tests ---

    #[test]
    fn embedded_task_structural_error_prefixed_with_step_path() {
        let bad_task = Task {
            id: "bad-id".into(), // kebab-case is invalid for Task id
            ..valid_task_entity("GoodId")
        };
        let wf = WorkflowDef {
            steps: vec![Step::Work(WorkStep {
                depends_on: None,
                definition: WorkStepDefinition::Task(bad_task),
            })],
            ..valid_workflow()
        };
        let errors = validate(&wf, &make_ctx());
        assert!(
            errors
                .iter()
                .any(|e| e.path.starts_with("steps[0].definition")),
            "Got paths: {:?}",
            errors.iter().map(|e| &e.path).collect::<Vec<_>>()
        );
    }

    #[test]
    fn structural_errors_prevent_semantic_validation() {
        // A workflow with a structural error (bad id) should not also report semantic errors.
        let wf = WorkflowDef {
            id: "bad-id".into(),
            states: vec![
                // Missing complete semantic — would be a semantic error if we got there
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
        let errors = validate(&wf, &make_ctx());
        // Should have structural error (bad id) but NOT semantic error (missing complete)
        assert!(errors.iter().any(|e| e.path == "id"));
        assert!(!errors.iter().any(|e| e.message.contains("complete")));
    }

    #[test]
    fn embedded_relay_error_prefixed_with_step_path() {
        // Relay with invalid id (structural) — error path must be prefixed with steps[0].definition
        let bad_relay = Relay {
            id: "bad-relay-id".into(), // kebab-case invalid for Relay
            name: "Legal Signoff".to_string(),
            description: None,
            purpose: "Delegate".to_string(),
            accountability: None,
            delegates_to: "LegalReview".to_string(),
            briefing: None,
            debriefing: None,
            state_map: {
                let mut m = HashMap::new();
                m.insert(
                    "Done".to_string(),
                    StateMapEntry {
                        maps_to: "Complete".to_string(),
                        semantic: Some(RelayStateSemantic::Complete),
                    },
                );
                m.insert(
                    "Active".to_string(),
                    StateMapEntry {
                        maps_to: "InProgress".to_string(),
                        semantic: None,
                    },
                );
                m
            },
            hooks: None,
            guidance: None,
            extensions: Extensions::default(),
        };
        let wf = WorkflowDef {
            steps: vec![Step::Work(WorkStep {
                depends_on: None,
                definition: WorkStepDefinition::Relay(bad_relay),
            })],
            ..valid_workflow()
        };
        let errors = validate(&wf, &make_ctx());
        assert!(
            errors
                .iter()
                .any(|e| e.path.starts_with("steps[0].definition")),
            "Got paths: {:?}",
            errors.iter().map(|e| &e.path).collect::<Vec<_>>()
        );
    }

    #[test]
    fn inline_workflow_errors_recurse_with_prefixed_path() {
        let bad_inner = WorkflowDef {
            id: "bad-inner-id".into(), // structural error in nested workflow
            ..valid_workflow()
        };
        let wf = WorkflowDef {
            steps: vec![Step::Work(WorkStep {
                depends_on: None,
                definition: WorkStepDefinition::Workflow(Box::new(bad_inner)),
            })],
            ..valid_workflow()
        };
        let errors = validate(&wf, &make_ctx());
        assert!(
            errors
                .iter()
                .any(|e| e.path.starts_with("steps[0].definition")),
            "Got paths: {:?}",
            errors.iter().map(|e| &e.path).collect::<Vec<_>>()
        );
    }

    // --- Task 1.1: Generic Step<S> and type alias construction ---

    #[test]
    fn workflow_step_is_generic_over_work_step_definition() {
        // Explicitly verifies the type is WorkflowDef<WorkStepDefinition> with Step<WorkStepDefinition>
        let step: Step<WorkStepDefinition> = Step::Work(WorkStep {
            depends_on: None,
            definition: WorkStepDefinition::Task(valid_task_entity("Proposal")),
        });
        assert_eq!(step.id(), "Proposal");
        let wf: Workflow = WorkflowDef {
            id: "GenericTest".into(),
            name: "Generic Test".to_string(),
            description: None,
            purpose: "Verify generic type".to_string(),
            accountability: base_raci(),
            steps: vec![step],
            states: two_states_with_complete(),
            hooks: None,
            guidance: None,
            extensions: Extensions::default(),
        };
        assert_eq!(wf.steps[0].id(), "Proposal");
    }

    #[test]
    fn shared_workflow_uses_step_type_aliases() {
        // Verifies SharedStep and SharedWorkStep aliases resolve correctly
        let ws: SharedWorkStep = WorkStep {
            depends_on: None,
            definition: SharedWorkStepDefinition::Task(valid_task_entity("Scope")),
        };
        let step: SharedStep = SharedStep::Work(ws);
        assert_eq!(step.id(), "Scope");
        let swf: SharedWorkflow = WorkflowDef {
            id: "AliasTest".into(),
            name: "Alias Test".to_string(),
            description: None,
            purpose: "Verify aliases".to_string(),
            accountability: base_raci(),
            steps: vec![step],
            states: two_states_with_complete(),
            hooks: None,
            guidance: None,
            extensions: Extensions::default(),
        };
        assert_eq!(swf.steps[0].id(), "Scope");
    }
}
