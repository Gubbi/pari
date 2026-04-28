//! User job: validation fires at the right moment.
//!
//! Validation rules apply at every lifecycle point that can introduce
//! invalid data — not just at insert. Setters run structural and
//! semantic validation against a candidate before swapping. Commit
//! re-runs cross-entity validation against fields the setter mutated
//! (cross-entity is not in the setter's contract). Each scenario pins
//! one tier × one moment.

use indexmap::IndexMap;
use pari::{
    entities::{
        task::Task,
        workflow::{Step, Workflow},
    },
    entity::{AnyEntityRef, EntityRef, TrackedEntity, WorkflowParent},
    error::{primitive::PrimitiveError, ActivityError},
    workspace::EntityClient,
};

use crate::{
    common::substrate::{run_with, SubstrateKind},
    fixtures::{
        artifact_kind::a_minimal_artifact_kind, role::a_minimal_role,
        task::a_minimal_task_with_parent, workflow::a_workflow_with_empty_steps,
    },
};

fn workflow_ref(id: &str) -> AnyEntityRef {
    AnyEntityRef::Workflow(EntityRef::new(id))
}

fn role_ref(id: &str) -> AnyEntityRef {
    AnyEntityRef::Role(EntityRef::new(id))
}

fn assert_validation_at(
    result: Result<(), ActivityError>,
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

/// Setter runs structural validation against the candidate. Swapping
/// in an empty name fails the `non_empty_str` rule before the field
/// changes.
#[tokio::test]
async fn setter_structural_validation_fires_at_setter_time() {
    run_with(SubstrateKind::InMemory, || async {
        EntityClient::insert(a_minimal_role("eng-lead"))
            .await
            .unwrap();
        EntityClient::persist().await.unwrap();
        let mut entity = EntityClient::checkout(role_ref("eng-lead")).await.unwrap();
        let TrackedEntity::Role(ref mut r) = entity else {
            panic!("expected Role")
        };
        let result = r.set_name(String::new()).await;
        assert_validation_at(result, "name", |e| {
            matches!(e, PrimitiveError::EmptyRequiredValue { .. })
        });
    })
    .await;
}

/// Setter runs semantic validation against the candidate. Swapping in
/// a `Step::Review` whose `on_reject` references a non-existent step
/// fails the `on_reject_valid` semantic rule.
#[tokio::test]
async fn setter_semantic_validation_fires_at_setter_time() {
    run_with(SubstrateKind::InMemory, || async {
        EntityClient::insert(a_minimal_role("eng-lead"))
            .await
            .unwrap();
        EntityClient::insert(a_workflow_with_empty_steps("DesignFlow", "eng-lead"))
            .await
            .unwrap();
        EntityClient::persist().await.unwrap();

        let mut entity = EntityClient::checkout(workflow_ref("DesignFlow"))
            .await
            .unwrap();
        let TrackedEntity::Workflow(ref mut wf) = entity else {
            panic!("expected Workflow")
        };
        let mut steps: IndexMap<String, Step> = IndexMap::new();
        steps.insert(
            "Review".to_string(),
            Step::Review {
                approver: vec![EntityRef::new("eng-lead")],
                on_reject: "DoesNotExist".to_string(),
            },
        );
        let result = wf.set_steps(steps).await;
        assert_validation_at(result, "steps", |e| {
            matches!(e, PrimitiveError::InvalidOnRejectTarget { .. })
        });
    })
    .await;
}

/// Cross-entity validation is not part of the setter's contract — it
/// runs at commit on fields the setter marked dirty. Pointing a step at
/// a task ref that does not exist accepts the setter, then fails
/// commit's `ref_check` on `steps`.
#[tokio::test]
async fn commit_cross_entity_validation_fires_for_setter_mutated_refs() {
    run_with(SubstrateKind::InMemory, || async {
        EntityClient::insert(a_minimal_role("eng-lead"))
            .await
            .unwrap();
        EntityClient::insert(a_minimal_artifact_kind("design-doc"))
            .await
            .unwrap();
        EntityClient::insert(a_workflow_with_empty_steps("DesignFlow", "eng-lead"))
            .await
            .unwrap();
        // A task that exists, then a step pointing at a different task
        // id that does NOT exist.
        EntityClient::insert(a_minimal_task_with_parent(
            "Real",
            WorkflowParent::Workflow(EntityRef::<Workflow>::new("DesignFlow")),
            "design-doc",
        ))
        .await
        .unwrap();
        EntityClient::persist().await.unwrap();

        let mut entity = EntityClient::checkout(workflow_ref("DesignFlow"))
            .await
            .unwrap();
        let TrackedEntity::Workflow(ref mut wf) = entity else {
            panic!("expected Workflow")
        };
        let parent = WorkflowParent::Workflow(EntityRef::<Workflow>::new("DesignFlow"));
        let mut steps: IndexMap<String, Step> = IndexMap::new();
        steps.insert(
            "Phantom".to_string(),
            Step::Task {
                entity_ref: EntityRef::<Task, _>::with_parent("Phantom", parent),
                depends_on: None,
            },
        );
        // Setter does not run cross-entity validation — accepts.
        wf.set_steps(steps).await.unwrap();
        // Commit re-runs cross-entity for dirty fields and rejects.
        let result = entity.commit().await;
        assert_validation_at(result, "steps", |e| {
            matches!(e, PrimitiveError::ReferencedEntityAbsent { .. })
        });
    })
    .await;
}
