//! User job: roll back a staged change before persist.
//!
//! `undo_commit` reverts pending writes in two distinct cases:
//! a freshly-added entity is removed entirely; a modified-but-not-yet-
//! persisted entity reverts to its prior persisted state. The
//! committed-but-not-persisted snapshot lives only in the store, so
//! both rollback paths run before the next `persist`.

use pari::{
    entities::role::Role,
    entity::{AnyEntityRef, EntityRef, TrackedEntity},
    error::{primitive::PrimitiveError, ActivityError},
    workspace::EntityClient,
};
use rstest::rstest;

use crate::{
    common::substrate::{run_with, SubstrateKind},
    fixtures::role::a_minimal_role,
};

fn role_ref(id: &str) -> AnyEntityRef {
    AnyEntityRef::Role(EntityRef::new(id))
}

fn role_typed(id: &str) -> EntityRef<Role> {
    EntityRef::new(id)
}

#[rstest]
#[case::in_memory(SubstrateKind::InMemory)]
#[case::repo(SubstrateKind::Repo)]
#[tokio::test]
async fn undo_commit_added_entity_removes_it(#[case] kind: SubstrateKind) {
    run_with(kind, || async {
        // Inserted but never persisted — purely an `added` entry.
        EntityClient::insert(a_minimal_role("eng-lead"))
            .await
            .unwrap();

        EntityClient::undo_commit(role_ref("eng-lead"))
            .await
            .unwrap();

        // Resolve fails — entity was never persisted and is no longer
        // in the store.
        let result = EntityClient::resolve(role_ref("eng-lead")).await;
        let err = result.err().expect("expected NonExistentData");
        assert!(
            matches!(
                &err,
                ActivityError::NonExistentData { cause, .. }
                    if matches!(cause, PrimitiveError::EntityNotFound { .. })
            ),
            "expected EntityNotFound, got: {err:?}"
        );
    })
    .await;
}

#[rstest]
#[case::in_memory(SubstrateKind::InMemory)]
#[case::repo(SubstrateKind::Repo)]
#[tokio::test]
async fn undo_commit_modified_entity_reverts_to_persisted(#[case] kind: SubstrateKind) {
    run_with(kind, || async {
        EntityClient::insert(a_minimal_role("eng-lead"))
            .await
            .unwrap();
        EntityClient::persist().await.unwrap();

        // Stage a modification, commit it (now `modified` in the store)
        // but do not persist — the on-disk / canonical state is still
        // the original.
        let mut role = EntityClient::checkout(role_typed("eng-lead"))
            .await
            .unwrap();
        role.set_name("Pending Change".to_string()).await.unwrap();
        role.commit().await.unwrap();

        EntityClient::undo_commit(role_ref("eng-lead"))
            .await
            .unwrap();

        // Resolve re-loads the persisted value.
        let resolved = EntityClient::resolve(role_ref("eng-lead")).await.unwrap();
        let TrackedEntity::Role(role) = resolved else {
            panic!("expected Role")
        };
        assert_eq!(role.name().await.unwrap(), "Minimal Role");
    })
    .await;
}
