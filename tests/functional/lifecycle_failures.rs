//! User job: lifecycle invariants are enforced.
//!
//! Operations issued at the wrong point — against missing entities, on
//! already-checked-out refs, with pending checkouts blocking persist —
//! fail with the matching `ActivityError` variant. Validation behavior
//! lives in `validation_failures.rs`; this file is purely about
//! lifecycle and store-state preconditions. Substrate-incidental, so
//! every scenario runs against `InMemorySubstrate`.

use pari::{
    entity::{AnyEntityRef, EntityRef, TrackedEntity},
    error::{primitive::PrimitiveError, ActivityError},
    workspace::EntityClient,
};

use crate::{
    common::substrate::{run_with, SubstrateKind},
    fixtures::role::a_minimal_role,
};

fn role_ref(id: &str) -> AnyEntityRef {
    AnyEntityRef::Role(EntityRef::new(id))
}

fn assert_activity_error<T>(
    result: Result<T, ActivityError>,
    matches: impl Fn(&ActivityError) -> bool,
) {
    let err = result.err().expect("expected an error");
    assert!(matches(&err), "unexpected error variant: {err:?}");
}

fn checkout_lifecycle(cause: impl Fn(&PrimitiveError) -> bool) -> impl Fn(&ActivityError) -> bool {
    move |e| matches!(e, ActivityError::CheckoutLifecycleViolation { cause: c, .. } if cause(c))
}

fn non_existent(cause: impl Fn(&PrimitiveError) -> bool) -> impl Fn(&ActivityError) -> bool {
    move |e| matches!(e, ActivityError::NonExistentData { cause: c, .. } if cause(c))
}

fn workspace_not_clean(cause: impl Fn(&PrimitiveError) -> bool) -> impl Fn(&ActivityError) -> bool {
    move |e| matches!(e, ActivityError::WorkspaceNotClean { cause: c, .. } if cause(c))
}

#[tokio::test]
async fn insert_duplicate_id_fails() {
    run_with(SubstrateKind::InMemory, || async {
        EntityClient::insert(a_minimal_role("eng-lead"))
            .await
            .unwrap();
        let result = EntityClient::insert(a_minimal_role("eng-lead")).await;
        assert_activity_error(
            result,
            checkout_lifecycle(|e| matches!(e, PrimitiveError::EntityAlreadyExists { .. })),
        );
    })
    .await;
}

#[tokio::test]
async fn resolve_missing_entity_fails() {
    run_with(SubstrateKind::InMemory, || async {
        let result = EntityClient::resolve(role_ref("missing")).await;
        assert_activity_error(
            result,
            non_existent(|e| matches!(e, PrimitiveError::EntityNotFound { .. })),
        );
    })
    .await;
}

#[tokio::test]
async fn checkout_missing_entity_fails() {
    run_with(SubstrateKind::InMemory, || async {
        let result = EntityClient::checkout(role_ref("missing")).await;
        assert_activity_error(
            result,
            non_existent(|e| matches!(e, PrimitiveError::EntityNotFound { .. })),
        );
    })
    .await;
}

#[tokio::test]
async fn checkout_already_checked_out_fails() {
    run_with(SubstrateKind::InMemory, || async {
        EntityClient::insert(a_minimal_role("eng-lead"))
            .await
            .unwrap();
        let _entity = EntityClient::checkout(role_ref("eng-lead")).await.unwrap();
        let result = EntityClient::checkout(role_ref("eng-lead")).await;
        assert_activity_error(
            result,
            checkout_lifecycle(|e| matches!(e, PrimitiveError::AlreadyCheckedOut { .. })),
        );
    })
    .await;
}

#[tokio::test]
async fn commit_without_checkout_fails() {
    run_with(SubstrateKind::InMemory, || async {
        EntityClient::insert(a_minimal_role("eng-lead"))
            .await
            .unwrap();
        // Resolve gives us a TrackedEntity, but no checkout has been
        // taken. commit must reject.
        let entity = EntityClient::resolve(role_ref("eng-lead")).await.unwrap();
        let result = entity.commit().await;
        assert_activity_error(
            result,
            checkout_lifecycle(|e| matches!(e, PrimitiveError::EntityNotCheckedOut { .. })),
        );
    })
    .await;
}

#[tokio::test]
async fn persist_with_pending_checkouts_fails() {
    run_with(SubstrateKind::InMemory, || async {
        EntityClient::insert(a_minimal_role("eng-lead"))
            .await
            .unwrap();
        let _checked_out = EntityClient::checkout(role_ref("eng-lead")).await.unwrap();
        let result = EntityClient::persist().await;
        assert_activity_error(
            result,
            workspace_not_clean(|e| matches!(e, PrimitiveError::PendingCheckouts { .. })),
        );
    })
    .await;
}

#[tokio::test]
async fn remove_checked_out_entity_fails() {
    run_with(SubstrateKind::InMemory, || async {
        EntityClient::insert(a_minimal_role("eng-lead"))
            .await
            .unwrap();
        let _checked_out = EntityClient::checkout(role_ref("eng-lead")).await.unwrap();
        let result = EntityClient::remove(role_ref("eng-lead")).await;
        assert_activity_error(
            result,
            checkout_lifecycle(|e| matches!(e, PrimitiveError::EntityStillCheckedOut { .. })),
        );
    })
    .await;
}

#[tokio::test]
async fn remove_missing_entity_fails() {
    run_with(SubstrateKind::InMemory, || async {
        let result = EntityClient::remove(role_ref("missing")).await;
        assert_activity_error(
            result,
            non_existent(|e| matches!(e, PrimitiveError::EntityNotFound { .. })),
        );
    })
    .await;
}

#[tokio::test]
async fn undo_commit_checked_out_entity_fails() {
    run_with(SubstrateKind::InMemory, || async {
        EntityClient::insert(a_minimal_role("eng-lead"))
            .await
            .unwrap();
        let _checked_out = EntityClient::checkout(role_ref("eng-lead")).await.unwrap();
        let result = EntityClient::undo_commit(role_ref("eng-lead")).await;
        assert_activity_error(
            result,
            checkout_lifecycle(|e| matches!(e, PrimitiveError::EntityStillCheckedOut { .. })),
        );
    })
    .await;
}

#[tokio::test]
async fn undo_commit_with_no_changes_fails() {
    run_with(SubstrateKind::InMemory, || async {
        EntityClient::insert(a_minimal_role("eng-lead"))
            .await
            .unwrap();
        EntityClient::persist().await.unwrap();
        // A persisted entity with no uncommitted changes has nothing to
        // undo.
        let result = EntityClient::undo_commit(role_ref("eng-lead")).await;
        assert_activity_error(
            result,
            checkout_lifecycle(|e| matches!(e, PrimitiveError::NoUncommittedChanges { .. })),
        );
    })
    .await;
}

#[tokio::test]
async fn unload_checked_out_entity_fails() {
    run_with(SubstrateKind::InMemory, || async {
        EntityClient::insert(a_minimal_role("eng-lead"))
            .await
            .unwrap();
        EntityClient::persist().await.unwrap();
        let _checked_out = EntityClient::checkout(role_ref("eng-lead")).await.unwrap();
        let result = EntityClient::unload(role_ref("eng-lead")).await;
        assert_activity_error(
            result,
            checkout_lifecycle(|e| matches!(e, PrimitiveError::EntityStillCheckedOut { .. })),
        );
    })
    .await;
}

#[tokio::test]
async fn unload_missing_entity_fails() {
    run_with(SubstrateKind::InMemory, || async {
        let result = EntityClient::unload(role_ref("missing")).await;
        assert_activity_error(
            result,
            non_existent(|e| matches!(e, PrimitiveError::EntityNotFound { .. })),
        );
    })
    .await;
}

#[tokio::test]
async fn unload_unsaved_entity_fails() {
    run_with(SubstrateKind::InMemory, || async {
        // Inserted but not yet persisted — has unsaved adds. Unload
        // would lose them.
        EntityClient::insert(a_minimal_role("eng-lead"))
            .await
            .unwrap();
        let result = EntityClient::unload(role_ref("eng-lead")).await;
        assert_activity_error(
            result,
            checkout_lifecycle(|e| matches!(e, PrimitiveError::EntityHasUnsavedChanges { .. })),
        );
    })
    .await;
}
