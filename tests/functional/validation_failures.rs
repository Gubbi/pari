//! User job: validation rejects invalid input.
//!
//! Comprehensive coverage across all three validation tiers —
//! structural, semantic, cross-entity. Each scenario asserts the
//! specific [`PrimitiveError`] kind expected at the field where the
//! offending rule fires; a generic "an error happened" assertion would
//! pass for the wrong reason. Validation behavior is substrate-
//! incidental, so each scenario runs against `InMemorySubstrate`.

use std::collections::HashMap;

use indexmap::IndexMap;
use pari::{
    entities::{
        artifact_kind::ArtifactKind,
        hook::Hook,
        relay::{Relay, StateMapEntry},
        role::Role,
        task::Task,
        team::{Team, TeamMember},
        workflow::{ReusableWorkflow, Step, Workflow},
    },
    entity::{EntityRef, WorkflowParent},
    error::{primitive::PrimitiveError, ActivityError},
    types::{
        Artifact, Extensions, HookCall, Raci, TaskSemantic, TaskStateEntry, WorkflowSemantic,
        WorkflowStateEntry, WorkflowTrigger,
    },
};

use crate::{
    common::substrate::{run_with, SubstrateKind},
    fixtures::{
        artifact_kind::a_minimal_artifact_kind,
        hook::{a_hook_with_required_input, a_minimal_hook},
        relay::a_minimal_relay,
        reusable_workflow::a_reusable_workflow_with_review_step,
        role::a_minimal_role,
        task::a_minimal_task_with_parent,
        team::{a_minimal_team, a_team_with_composition, a_team_with_members},
        workflow::a_workflow_with_empty_steps,
    },
};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn assert_validation_error<T: std::fmt::Debug>(
    result: Result<T, ActivityError>,
    field: &str,
    matches: impl Fn(&PrimitiveError) -> bool,
) {
    let err = result.expect_err("expected ValidationFailed");
    let cause = match &err {
        ActivityError::ValidationFailed { cause, .. } => cause,
        _ => panic!("expected ValidationFailed, got: {err:?}"),
    };
    let errors = match cause {
        PrimitiveError::FieldValidationError { errors, .. } => errors,
        _ => panic!("expected FieldValidationError, got: {cause:?}"),
    };
    let field_errors = errors
        .get(field)
        .unwrap_or_else(|| panic!("expected errors at field '{field}', got: {errors:?}"));
    assert!(
        field_errors.iter().any(matches),
        "expected matching PrimitiveError at '{field}', got: {field_errors:?}"
    );
}

fn naming_violation(rule: &str) -> impl Fn(&PrimitiveError) -> bool + '_ {
    move |e| {
        matches!(
            e,
            PrimitiveError::NamingFormatViolation { rule_kind, .. } if rule_kind == rule
        )
    }
}

fn referenced_entity_absent(e: &PrimitiveError) -> bool {
    matches!(e, PrimitiveError::ReferencedEntityAbsent { .. })
}

fn workflow_graph_inconsistency(reason: &str) -> impl Fn(&PrimitiveError) -> bool + '_ {
    move |e| {
        matches!(
            e,
            PrimitiveError::WorkflowGraphInconsistency { reason: r, .. } if r == reason
        )
    }
}

// ---------------------------------------------------------------------------
// Inline builders for entities that fail validation
// ---------------------------------------------------------------------------

fn role(id: &str, name: &str, extensions: Extensions) -> Role {
    Role {
        entity_ref: EntityRef::new(id),
        name: name.to_string(),
        description: None,
        purpose: "test".to_string(),
        traits: None,
        extensions,
    }
}

fn workflow(
    id: &str,
    raci: Raci,
    states: Vec<WorkflowStateEntry>,
    steps: IndexMap<String, Step>,
    intercepts: Option<HashMap<WorkflowTrigger, HookCall>>,
) -> Workflow {
    Workflow {
        entity_ref: EntityRef::new(id),
        name: "Bad Workflow".to_string(),
        description: None,
        purpose: "test".to_string(),
        raci,
        states,
        steps,
        intercepts,
        guidance: None,
        extensions: Default::default(),
    }
}

fn canonical_raci(role_id: &str) -> Raci {
    Raci {
        responsible: vec![EntityRef::new(role_id)],
        accountable: EntityRef::new(role_id),
        consulted: None,
        informed: None,
    }
}

fn three_state() -> Vec<WorkflowStateEntry> {
    vec![
        WorkflowStateEntry {
            id: "InProgress".to_string(),
            description: "wip".to_string(),
            semantic: None,
        },
        WorkflowStateEntry {
            id: "InReview".to_string(),
            description: "review".to_string(),
            semantic: Some(WorkflowSemantic::Reviewing),
        },
        WorkflowStateEntry {
            id: "Done".to_string(),
            description: "done".to_string(),
            semantic: Some(WorkflowSemantic::Done),
        },
    ]
}

fn two_state_done() -> Vec<WorkflowStateEntry> {
    vec![
        WorkflowStateEntry {
            id: "InProgress".to_string(),
            description: "wip".to_string(),
            semantic: None,
        },
        WorkflowStateEntry {
            id: "Done".to_string(),
            description: "done".to_string(),
            semantic: Some(WorkflowSemantic::Done),
        },
    ]
}

// ===========================================================================
// Structural
// ===========================================================================

#[tokio::test]
async fn role_with_invalid_id_fails() {
    run_with(SubstrateKind::InMemory, |workspace| async move {
        let bad = role("InvalidId", "ok", Default::default());
        assert_validation_error(
            workspace.insert(bad).await,
            "entity_ref",
            naming_violation("kebab_case"),
        );
    })
    .await;
}

#[tokio::test]
async fn workflow_with_invalid_id_fails() {
    run_with(SubstrateKind::InMemory, |workspace| async move {
        workspace.insert(a_minimal_role("eng-lead")).await.unwrap();
        let wf = workflow(
            "design-flow",
            canonical_raci("eng-lead"),
            three_state(),
            IndexMap::new(),
            None,
        );
        assert_validation_error(
            workspace.insert(wf).await,
            "entity_ref",
            naming_violation("pascal_case"),
        );
    })
    .await;
}

#[tokio::test]
async fn role_with_empty_name_fails() {
    run_with(SubstrateKind::InMemory, |workspace| async move {
        let bad = role("eng-lead", "", Default::default());
        assert_validation_error(workspace.insert(bad).await, "name", |e| {
            matches!(e, PrimitiveError::EmptyRequiredValue { .. })
        });
    })
    .await;
}

#[tokio::test]
async fn workflow_with_too_few_states_fails() {
    run_with(SubstrateKind::InMemory, |workspace| async move {
        workspace.insert(a_minimal_role("eng-lead")).await.unwrap();
        let states = vec![WorkflowStateEntry {
            id: "Done".to_string(),
            description: "done".to_string(),
            semantic: Some(WorkflowSemantic::Done),
        }];
        let wf = workflow(
            "DesignFlow",
            canonical_raci("eng-lead"),
            states,
            IndexMap::new(),
            None,
        );
        assert_validation_error(
            workspace.insert(wf).await,
            "states",
            |e| matches!(e, PrimitiveError::MalformedCollectionValue { rule_kind, .. } if rule_kind == "min_length"),
        );
    })
    .await;
}

#[tokio::test]
async fn workflow_with_invalid_state_id_fails() {
    run_with(SubstrateKind::InMemory, |workspace| async move {
        workspace.insert(a_minimal_role("eng-lead")).await.unwrap();
        let states = vec![
            WorkflowStateEntry {
                id: "in-progress".to_string(),
                description: "wip".to_string(),
                semantic: None,
            },
            WorkflowStateEntry {
                id: "Done".to_string(),
                description: "done".to_string(),
                semantic: Some(WorkflowSemantic::Done),
            },
        ];
        let wf = workflow(
            "DesignFlow",
            canonical_raci("eng-lead"),
            states,
            IndexMap::new(),
            None,
        );
        assert_validation_error(
            workspace.insert(wf).await,
            "states",
            naming_violation("pascal_case"),
        );
    })
    .await;
}

#[tokio::test]
async fn workflow_with_no_done_state_fails() {
    run_with(SubstrateKind::InMemory, |workspace| async move {
        workspace.insert(a_minimal_role("eng-lead")).await.unwrap();
        let states = vec![
            WorkflowStateEntry {
                id: "InProgress".to_string(),
                description: "wip".to_string(),
                semantic: None,
            },
            WorkflowStateEntry {
                id: "Reviewing".to_string(),
                description: "review".to_string(),
                semantic: Some(WorkflowSemantic::Reviewing),
            },
        ];
        let wf = workflow(
            "DesignFlow",
            canonical_raci("eng-lead"),
            states,
            IndexMap::new(),
            None,
        );
        assert_validation_error(
            workspace.insert(wf).await,
            "states",
            workflow_graph_inconsistency("missing_done_semantic"),
        );
    })
    .await;
}

#[tokio::test]
async fn workflow_with_all_done_states_fails() {
    run_with(SubstrateKind::InMemory, |workspace| async move {
        workspace.insert(a_minimal_role("eng-lead")).await.unwrap();
        let states = vec![
            WorkflowStateEntry {
                id: "DoneA".to_string(),
                description: "done".to_string(),
                semantic: Some(WorkflowSemantic::Done),
            },
            WorkflowStateEntry {
                id: "DoneB".to_string(),
                description: "done".to_string(),
                semantic: Some(WorkflowSemantic::Done),
            },
        ];
        let wf = workflow(
            "DesignFlow",
            canonical_raci("eng-lead"),
            states,
            IndexMap::new(),
            None,
        );
        assert_validation_error(
            workspace.insert(wf).await,
            "states",
            workflow_graph_inconsistency("all_done_states"),
        );
    })
    .await;
}

#[tokio::test]
async fn workflow_with_duplicate_state_ids_fails() {
    run_with(SubstrateKind::InMemory, |workspace| async move {
        workspace.insert(a_minimal_role("eng-lead")).await.unwrap();
        let states = vec![
            WorkflowStateEntry {
                id: "InProgress".to_string(),
                description: "wip".to_string(),
                semantic: None,
            },
            WorkflowStateEntry {
                id: "InProgress".to_string(),
                description: "duplicate".to_string(),
                semantic: None,
            },
            WorkflowStateEntry {
                id: "Done".to_string(),
                description: "done".to_string(),
                semantic: Some(WorkflowSemantic::Done),
            },
        ];
        let wf = workflow(
            "DesignFlow",
            canonical_raci("eng-lead"),
            states,
            IndexMap::new(),
            None,
        );
        assert_validation_error(
            workspace.insert(wf).await,
            "states",
            |e| matches!(e, PrimitiveError::DuplicateEntryViolation { rule_kind, .. } if rule_kind == "unique"),
        );
    })
    .await;
}

#[tokio::test]
async fn workflow_with_invalid_step_key_fails() {
    run_with(SubstrateKind::InMemory, |workspace| async move {
        workspace.insert(a_minimal_role("eng-lead")).await.unwrap();
        let mut steps: IndexMap<String, Step> = IndexMap::new();
        steps.insert(
            "review".to_string(),
            Step::Review {
                approver: vec![EntityRef::new("eng-lead")],
                on_reject: "review".to_string(),
            },
        );
        let wf = workflow(
            "DesignFlow",
            canonical_raci("eng-lead"),
            three_state(),
            steps,
            None,
        );
        assert_validation_error(
            workspace.insert(wf).await,
            "steps",
            naming_violation("pascal_case"),
        );
    })
    .await;
}

#[tokio::test]
async fn team_with_duplicate_member_handle_fails() {
    run_with(SubstrateKind::InMemory, |workspace| async move {
        workspace.insert(a_minimal_role("eng-lead")).await.unwrap();
        let team = a_team_with_members(
            "eng",
            &[("@alice", "eng-lead"), ("@alice", "eng-lead")],
        );
        assert_validation_error(
            workspace.insert(team).await,
            "members",
            |e| matches!(e, PrimitiveError::DuplicateEntryViolation { rule_kind, .. } if rule_kind == "unique"),
        );
    })
    .await;
}

#[tokio::test]
async fn team_with_invalid_member_handle_fails() {
    run_with(SubstrateKind::InMemory, |workspace| async move {
        workspace.insert(a_minimal_role("eng-lead")).await.unwrap();
        let team = Team {
            entity_ref: EntityRef::new("eng"),
            name: "Eng".to_string(),
            description: None,
            members: Some(vec![TeamMember {
                handle: "alice".to_string(), // missing '@'
                role: EntityRef::new("eng-lead"),
            }]),
            include: None,
            import: None,
            extensions: Default::default(),
        };
        assert_validation_error(
            workspace.insert(team).await,
            "members",
            naming_violation("handle_format"),
        );
    })
    .await;
}

#[tokio::test]
async fn team_with_duplicate_include_fails() {
    run_with(SubstrateKind::InMemory, |workspace| async move {
        workspace.insert(a_minimal_role("eng-lead")).await.unwrap();
        workspace.insert(a_minimal_team("platform")).await.unwrap();
        let team = a_team_with_composition(
            "eng",
            &[("platform", "eng-lead"), ("platform", "eng-lead")],
            &[],
        );
        assert_validation_error(
            workspace.insert(team).await,
            "include",
            |e| matches!(e, PrimitiveError::DuplicateEntryViolation { rule_kind, .. } if rule_kind == "unique"),
        );
    })
    .await;
}

#[tokio::test]
async fn raci_with_empty_responsible_fails() {
    run_with(SubstrateKind::InMemory, |workspace| async move {
        workspace.insert(a_minimal_role("eng-lead")).await.unwrap();
        let raci = Raci {
            responsible: vec![],
            accountable: EntityRef::new("eng-lead"),
            consulted: None,
            informed: None,
        };
        let wf = workflow("DesignFlow", raci, three_state(), IndexMap::new(), None);
        assert_validation_error(
            workspace.insert(wf).await,
            "raci",
            |e| matches!(e, PrimitiveError::EmptyRequiredValue { rule_kind, .. } if rule_kind == "raci_structural"),
        );
    })
    .await;
}

#[tokio::test]
async fn relay_with_invalid_state_map_key_fails() {
    run_with(SubstrateKind::InMemory, |workspace| async move {
        workspace.insert(a_minimal_role("eng-lead")).await.unwrap();
        workspace.insert(a_minimal_role("approver")).await.unwrap();
        workspace
            .insert(a_reusable_workflow_with_review_step(
                "ApprovalLoop",
                "eng-lead",
                "approver",
            ))
            .await
            .unwrap();
        workspace
            .insert(a_workflow_with_empty_steps("DesignFlow", "eng-lead"))
            .await
            .unwrap();

        let mut state_map = HashMap::new();
        state_map.insert(
            "pending".to_string(), // lowercase — not PascalCase
            StateMapEntry {
                maps_to: "InProgress".to_string(),
                description: None,
                semantic: None,
            },
        );
        let parent = WorkflowParent::Workflow(EntityRef::<Workflow>::new("DesignFlow"));
        let bad = Relay {
            entity_ref: EntityRef::with_parent("Handoff", parent),
            name: "Bad".to_string(),
            description: None,
            purpose: "test".to_string(),
            raci: None,
            delegates_to: EntityRef::<ReusableWorkflow>::new("ApprovalLoop"),
            briefing: None,
            debriefing: None,
            state_map,
            intercepts: None,
            guidance: None,
            extensions: Default::default(),
        };
        assert_validation_error(
            workspace.insert(bad).await,
            "state_map",
            naming_violation("pascal_case"),
        );
    })
    .await;
}

#[tokio::test]
async fn relay_with_empty_state_map_fails() {
    run_with(SubstrateKind::InMemory, |workspace| async move {
        workspace.insert(a_minimal_role("eng-lead")).await.unwrap();
        workspace.insert(a_minimal_role("approver")).await.unwrap();
        workspace
            .insert(a_reusable_workflow_with_review_step(
                "ApprovalLoop",
                "eng-lead",
                "approver",
            ))
            .await
            .unwrap();
        workspace
            .insert(a_workflow_with_empty_steps("DesignFlow", "eng-lead"))
            .await
            .unwrap();

        let parent = WorkflowParent::Workflow(EntityRef::<Workflow>::new("DesignFlow"));
        let bad = Relay {
            entity_ref: EntityRef::with_parent("Handoff", parent),
            name: "Bad".to_string(),
            description: None,
            purpose: "test".to_string(),
            raci: None,
            delegates_to: EntityRef::<ReusableWorkflow>::new("ApprovalLoop"),
            briefing: None,
            debriefing: None,
            state_map: HashMap::new(),
            intercepts: None,
            guidance: None,
            extensions: Default::default(),
        };
        assert_validation_error(
            workspace.insert(bad).await,
            "state_map",
            |e| matches!(e, PrimitiveError::MalformedCollectionValue { rule_kind, .. } if rule_kind == "non_empty"),
        );
    })
    .await;
}

#[tokio::test]
async fn task_with_empty_instructions_fails() {
    run_with(SubstrateKind::InMemory, |workspace| async move {
        workspace.insert(a_minimal_role("eng-lead")).await.unwrap();
        workspace
            .insert(a_minimal_artifact_kind("design-doc"))
            .await
            .unwrap();
        workspace
            .insert(a_workflow_with_empty_steps("DesignFlow", "eng-lead"))
            .await
            .unwrap();

        let parent = WorkflowParent::Workflow(EntityRef::<Workflow>::new("DesignFlow"));
        let bad = Task {
            entity_ref: EntityRef::with_parent("Design", parent),
            name: "Bad".to_string(),
            description: None,
            purpose: "test".to_string(),
            instructions: vec![],
            criteria: vec!["criterion".to_string()],
            raci: None,
            artifact: Artifact {
                kind: EntityRef::<ArtifactKind>::new("design-doc"),
                template: Some("# tpl\n".to_string()),
            },
            states: vec![
                TaskStateEntry {
                    id: "InProgress".to_string(),
                    description: "wip".to_string(),
                    semantic: None,
                },
                TaskStateEntry {
                    id: "Done".to_string(),
                    description: "done".to_string(),
                    semantic: Some(TaskSemantic::Done),
                },
            ],
            intercepts: None,
            guidance: None,
            extensions: Default::default(),
        };
        assert_validation_error(
            workspace.insert(bad).await,
            "instructions",
            |e| matches!(e, PrimitiveError::MalformedCollectionValue { rule_kind, .. } if rule_kind == "non_empty"),
        );
    })
    .await;
}

// ===========================================================================
// Semantic
// ===========================================================================

#[tokio::test]
async fn workflow_step_with_invalid_on_reject_fails() {
    run_with(SubstrateKind::InMemory, |workspace| async move {
        workspace.insert(a_minimal_role("eng-lead")).await.unwrap();
        let mut steps: IndexMap<String, Step> = IndexMap::new();
        steps.insert(
            "Review".to_string(),
            Step::Review {
                approver: vec![EntityRef::new("eng-lead")],
                on_reject: "DoesNotExist".to_string(),
            },
        );
        let wf = workflow(
            "DesignFlow",
            canonical_raci("eng-lead"),
            three_state(),
            steps,
            None,
        );
        assert_validation_error(workspace.insert(wf).await, "steps", |e| {
            matches!(e, PrimitiveError::InvalidOnRejectTarget { .. })
        });
    })
    .await;
}

#[tokio::test]
async fn workflow_step_with_invalid_depends_on_fails() {
    run_with(SubstrateKind::InMemory, |workspace| async move {
        workspace.insert(a_minimal_role("eng-lead")).await.unwrap();
        workspace
            .insert(a_minimal_artifact_kind("design-doc"))
            .await
            .unwrap();
        workspace
            .insert(a_workflow_with_empty_steps("DesignFlow", "eng-lead"))
            .await
            .unwrap();
        workspace
            .insert(a_minimal_task_with_parent(
                "Design",
                WorkflowParent::Workflow(EntityRef::<Workflow>::new("DesignFlow")),
                "design-doc",
            ))
            .await
            .unwrap();

        let parent = WorkflowParent::Workflow(EntityRef::<Workflow>::new("DesignFlow"));
        let task_ref = EntityRef::<Task, _>::with_parent("Design", parent);
        let mut steps: IndexMap<String, Step> = IndexMap::new();
        steps.insert(
            "Design".to_string(),
            Step::Task {
                entity_ref: task_ref,
                depends_on: Some(vec!["Missing".to_string()]),
            },
        );
        let mut wf = workspace
            .checkout(EntityRef::<Workflow>::new("DesignFlow"))
            .await
            .unwrap();
        let result = wf.set_steps(steps).await;
        assert_validation_error(result, "steps", |e| {
            matches!(e, PrimitiveError::IllegalDependencyReference { .. })
        });
    })
    .await;
}

#[tokio::test]
async fn workflow_step_with_forward_depends_on_fails() {
    run_with(SubstrateKind::InMemory, |workspace| async move {
        workspace.insert(a_minimal_role("eng-lead")).await.unwrap();
        workspace
            .insert(a_minimal_artifact_kind("design-doc"))
            .await
            .unwrap();
        workspace
            .insert(a_workflow_with_empty_steps("DesignFlow", "eng-lead"))
            .await
            .unwrap();
        workspace
            .insert(a_minimal_task_with_parent(
                "First",
                WorkflowParent::Workflow(EntityRef::<Workflow>::new("DesignFlow")),
                "design-doc",
            ))
            .await
            .unwrap();
        workspace
            .insert(a_minimal_task_with_parent(
                "Second",
                WorkflowParent::Workflow(EntityRef::<Workflow>::new("DesignFlow")),
                "design-doc",
            ))
            .await
            .unwrap();

        let parent = WorkflowParent::Workflow(EntityRef::<Workflow>::new("DesignFlow"));
        let mut steps: IndexMap<String, Step> = IndexMap::new();
        // First step depends on Second — forward reference, illegal.
        steps.insert(
            "First".to_string(),
            Step::Task {
                entity_ref: EntityRef::<Task, _>::with_parent("First", parent.clone()),
                depends_on: Some(vec!["Second".to_string()]),
            },
        );
        steps.insert(
            "Second".to_string(),
            Step::Task {
                entity_ref: EntityRef::<Task, _>::with_parent("Second", parent),
                depends_on: None,
            },
        );

        let mut wf = workspace
            .checkout(EntityRef::<Workflow>::new("DesignFlow"))
            .await
            .unwrap();
        let result = wf.set_steps(steps).await;
        assert_validation_error(result, "steps", |e| {
            matches!(e, PrimitiveError::IllegalDependencyReference { .. })
        });
    })
    .await;
}

#[tokio::test]
async fn workflow_with_review_step_missing_reviewing_state_fails() {
    run_with(SubstrateKind::InMemory, |workspace| async move {
        workspace.insert(a_minimal_role("eng-lead")).await.unwrap();
        let mut steps: IndexMap<String, Step> = IndexMap::new();
        steps.insert(
            "Review".to_string(),
            Step::Review {
                approver: vec![EntityRef::new("eng-lead")],
                on_reject: "Review".to_string(),
            },
        );
        let wf = workflow(
            "DesignFlow",
            canonical_raci("eng-lead"),
            two_state_done(), // no Reviewing semantic
            steps,
            None,
        );
        assert_validation_error(
            workspace.insert(wf).await,
            "steps",
            workflow_graph_inconsistency("missing_reviewing_semantic"),
        );
    })
    .await;
}

// ===========================================================================
// Cross-entity
// ===========================================================================

#[tokio::test]
async fn workflow_referencing_missing_role_fails() {
    run_with(SubstrateKind::InMemory, |workspace| async move {
        let wf = workflow(
            "DesignFlow",
            canonical_raci("missing-role"),
            three_state(),
            IndexMap::new(),
            None,
        );
        assert_validation_error(workspace.insert(wf).await, "raci", referenced_entity_absent);
    })
    .await;
}

#[tokio::test]
async fn embedded_entity_with_missing_parent_fails() {
    run_with(SubstrateKind::InMemory, |workspace| async move {
        workspace.insert(a_minimal_role("eng-lead")).await.unwrap();
        workspace
            .insert(a_minimal_artifact_kind("design-doc"))
            .await
            .unwrap();
        let parent = WorkflowParent::Workflow(EntityRef::<Workflow>::new("Phantom"));
        let task = a_minimal_task_with_parent("Design", parent, "design-doc");
        assert_validation_error(
            workspace.insert(task).await,
            "entity_ref",
            referenced_entity_absent,
        );
    })
    .await;
}

#[tokio::test]
async fn workflow_intercept_referencing_missing_hook_fails() {
    run_with(SubstrateKind::InMemory, |workspace| async move {
        workspace.insert(a_minimal_role("eng-lead")).await.unwrap();
        let mut intercepts = HashMap::new();
        intercepts.insert(
            WorkflowTrigger::OnDone,
            HookCall {
                hook: EntityRef::<Hook>::new("missing-hook"),
                with: None,
            },
        );
        let wf = workflow(
            "DesignFlow",
            canonical_raci("eng-lead"),
            three_state(),
            IndexMap::new(),
            Some(intercepts),
        );
        assert_validation_error(
            workspace.insert(wf).await,
            "intercepts",
            referenced_entity_absent,
        );
    })
    .await;
}

#[tokio::test]
async fn workflow_intercept_missing_required_input_fails() {
    run_with(SubstrateKind::InMemory, |workspace| async move {
        workspace.insert(a_minimal_role("eng-lead")).await.unwrap();
        workspace
            .insert(a_hook_with_required_input("summary-hook", "summary"))
            .await
            .unwrap();
        let mut intercepts = HashMap::new();
        intercepts.insert(
            WorkflowTrigger::OnDone,
            HookCall {
                hook: EntityRef::<Hook>::new("summary-hook"),
                with: None, // required input not bound
            },
        );
        let wf = workflow(
            "DesignFlow",
            canonical_raci("eng-lead"),
            three_state(),
            IndexMap::new(),
            Some(intercepts),
        );
        assert_validation_error(
            workspace.insert(wf).await,
            "intercepts",
            |e| matches!(e, PrimitiveError::EmptyRequiredValue { rule_kind, .. } if rule_kind == "required_input_missing"),
        );
    })
    .await;
}

#[tokio::test]
async fn workflow_intercept_unknown_input_key_fails() {
    run_with(SubstrateKind::InMemory, |workspace| async move {
        workspace.insert(a_minimal_role("eng-lead")).await.unwrap();
        workspace
            .insert(a_minimal_hook("on-done-hook"))
            .await
            .unwrap();
        let mut bindings = HashMap::new();
        bindings.insert("unknown".to_string(), "value".to_string());
        let mut intercepts = HashMap::new();
        intercepts.insert(
            WorkflowTrigger::OnDone,
            HookCall {
                hook: EntityRef::<Hook>::new("on-done-hook"),
                with: Some(bindings),
            },
        );
        let wf = workflow(
            "DesignFlow",
            canonical_raci("eng-lead"),
            three_state(),
            IndexMap::new(),
            Some(intercepts),
        );
        assert_validation_error(
            workspace.insert(wf).await,
            "intercepts",
            referenced_entity_absent,
        );
    })
    .await;
}

#[tokio::test]
async fn relay_state_map_referencing_missing_state_fails() {
    run_with(SubstrateKind::InMemory, |workspace| async move {
        workspace.insert(a_minimal_role("eng-lead")).await.unwrap();
        workspace.insert(a_minimal_role("approver")).await.unwrap();
        workspace
            .insert(a_reusable_workflow_with_review_step(
                "ApprovalLoop",
                "eng-lead",
                "approver",
            ))
            .await
            .unwrap();
        workspace
            .insert(a_workflow_with_empty_steps("DesignFlow", "eng-lead"))
            .await
            .unwrap();

        let mut state_map = HashMap::new();
        state_map.insert(
            "Pending".to_string(),
            StateMapEntry {
                maps_to: "DoesNotExist".to_string(),
                description: None,
                semantic: None,
            },
        );
        let parent = WorkflowParent::Workflow(EntityRef::<Workflow>::new("DesignFlow"));
        let bad = Relay {
            entity_ref: EntityRef::with_parent("Handoff", parent),
            name: "Bad".to_string(),
            description: None,
            purpose: "test".to_string(),
            raci: None,
            delegates_to: EntityRef::<ReusableWorkflow>::new("ApprovalLoop"),
            briefing: None,
            debriefing: None,
            state_map,
            intercepts: None,
            guidance: None,
            extensions: Default::default(),
        };
        assert_validation_error(
            workspace.insert(bad).await,
            "state_map",
            |e| matches!(
                e,
                PrimitiveError::WorkflowGraphInconsistency { reason: r, .. } if r.contains("maps_to")
            ) || referenced_entity_absent(e),
        );
    })
    .await;
}

#[tokio::test]
async fn team_include_cycle_fails() {
    run_with(SubstrateKind::InMemory, |workspace| async move {
        workspace.insert(a_minimal_role("eng-lead")).await.unwrap();
        // team-b includes team-a (no cycle yet).
        workspace.insert(a_minimal_team("team-a")).await.unwrap();
        workspace
            .insert(a_team_with_composition(
                "team-b",
                &[("team-a", "eng-lead")],
                &[],
            ))
            .await
            .unwrap();
        workspace.persist().await.unwrap();

        // Now modify team-a to include team-b — closes the cycle.
        let mut team = workspace
            .checkout(EntityRef::<Team>::new("team-a"))
            .await
            .unwrap();
        // Cross-entity (cycle) runs at commit, not setter; setter should
        // succeed. The cycle is detected on commit.
        team.set_include(Some(vec![(
            EntityRef::new("team-b"),
            EntityRef::new("eng-lead"),
        )]))
        .await
        .unwrap();
        let commit_result = team.commit().await;
        assert_validation_error(
            commit_result,
            "include",
            workflow_graph_inconsistency("include_cycle"),
        );
    })
    .await;
}

#[tokio::test]
async fn team_import_cycle_fails() {
    run_with(SubstrateKind::InMemory, |workspace| async move {
        workspace.insert(a_minimal_role("eng-lead")).await.unwrap();
        workspace.insert(a_minimal_team("team-a")).await.unwrap();
        workspace
            .insert(a_team_with_composition("team-b", &[], &["team-a"]))
            .await
            .unwrap();
        workspace.persist().await.unwrap();

        let mut team = workspace
            .checkout(EntityRef::<Team>::new("team-a"))
            .await
            .unwrap();
        team.set_import(Some(vec![EntityRef::new("team-b")]))
            .await
            .unwrap();
        let commit_result = team.commit().await;
        assert_validation_error(
            commit_result,
            "import",
            workflow_graph_inconsistency("import_cycle"),
        );
    })
    .await;
}

#[tokio::test]
async fn reusable_workflow_with_relay_step_fails() {
    run_with(SubstrateKind::InMemory, |workspace| async move {
        workspace.insert(a_minimal_role("eng-lead")).await.unwrap();
        workspace.insert(a_minimal_role("approver")).await.unwrap();
        // Inner reusable workflow + relay (under it would be illegal,
        // but we route through a parent workflow first to construct a
        // relay entity at all).
        workspace
            .insert(a_reusable_workflow_with_review_step(
                "Inner", "eng-lead", "approver",
            ))
            .await
            .unwrap();
        workspace
            .insert(a_workflow_with_empty_steps("DesignFlow", "eng-lead"))
            .await
            .unwrap();
        workspace
            .insert(a_minimal_relay(
                "Handoff",
                "DesignFlow",
                "eng-lead",
                "Inner",
            ))
            .await
            .unwrap();
        workspace.persist().await.unwrap();

        // Build a ReusableWorkflow whose step list contains Step::Relay —
        // construct directly because the fixture only does Review steps.
        let parent = WorkflowParent::Workflow(EntityRef::<Workflow>::new("DesignFlow"));
        let relay_ref = EntityRef::<Relay, _>::with_parent("Handoff", parent);
        let mut steps: IndexMap<String, Step> = IndexMap::new();
        steps.insert(
            "Handoff".to_string(),
            Step::Relay {
                entity_ref: relay_ref,
                depends_on: None,
            },
        );
        let bad = ReusableWorkflow {
            entity_ref: EntityRef::new("Outer"),
            name: "Outer".to_string(),
            description: None,
            purpose: "test".to_string(),
            raci: canonical_raci("eng-lead"),
            states: three_state(),
            steps,
            intercepts: None,
            guidance: None,
            extensions: Default::default(),
        };
        assert_validation_error(
            workspace.insert(bad).await,
            "steps",
            workflow_graph_inconsistency("relay_in_tree"),
        );
    })
    .await;
}

#[tokio::test]
async fn task_with_missing_artifact_kind_fails() {
    run_with(SubstrateKind::InMemory, |workspace| async move {
        workspace.insert(a_minimal_role("eng-lead")).await.unwrap();
        workspace
            .insert(a_workflow_with_empty_steps("DesignFlow", "eng-lead"))
            .await
            .unwrap();

        // artifact_kind "missing-kind" is never inserted.
        let parent = WorkflowParent::Workflow(EntityRef::<Workflow>::new("DesignFlow"));
        let task = a_minimal_task_with_parent("Design", parent, "missing-kind");
        assert_validation_error(
            workspace.insert(task).await,
            "artifact",
            referenced_entity_absent,
        );
    })
    .await;
}

#[tokio::test]
async fn team_member_referencing_missing_role_fails() {
    run_with(SubstrateKind::InMemory, |workspace| async move {
        // No role inserted; member references "missing-role".
        let team = a_team_with_members("eng", &[("@alice", "missing-role")]);
        assert_validation_error(
            workspace.insert(team).await,
            "members",
            referenced_entity_absent,
        );
    })
    .await;
}
