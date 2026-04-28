//! User job: abandon an in-progress edit.
//!
//! `undo_checkout` consumes the delegate, drops any pending mutations,
//! and releases the per-ref checkout so a fresh `checkout` succeeds.

use pari::{
    entities::role::Role,
    entity::{AnyEntityRef, EntityRef, TrackedEntity},
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
async fn undo_checkout_discards_changes_and_releases_lock(#[case] kind: SubstrateKind) {
    run_with(kind, || async {
        EntityClient::insert(a_minimal_role("eng-lead"))
            .await
            .unwrap();
        EntityClient::persist().await.unwrap();

        // Stage a change, then abandon it.
        let mut role = EntityClient::checkout(role_typed("eng-lead"))
            .await
            .unwrap();
        role.set_name("Abandoned Change".to_string()).await.unwrap();
        role.undo_checkout().await.unwrap();

        // The change is gone — resolve sees the original.
        let resolved = EntityClient::resolve(role_ref("eng-lead")).await.unwrap();
        let TrackedEntity::Role(role) = resolved else {
            panic!("expected Role")
        };
        assert_eq!(role.name().await.unwrap(), "Minimal Role");

        // The lock released — a fresh checkout succeeds and can commit.
        let mut role = EntityClient::checkout(role_typed("eng-lead"))
            .await
            .unwrap();
        role.set_name("Real Change".to_string()).await.unwrap();
        role.commit().await.unwrap();
        EntityClient::persist().await.unwrap();

        let resolved = EntityClient::resolve(role_ref("eng-lead")).await.unwrap();
        let TrackedEntity::Role(role) = resolved else {
            panic!("expected Role")
        };
        assert_eq!(role.name().await.unwrap(), "Real Change");
    })
    .await;
}
