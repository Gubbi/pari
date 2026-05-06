//! User job: abandon an in-progress edit.
//!
//! `undo_checkout` consumes the editor, drops any pending mutations,
//! and releases the per-ref checkout so a fresh `checkout` succeeds.

use pari::{entities::role::Role, entity::EntityRef};
use rstest::rstest;

use crate::{
    common::substrate::{run_with, SubstrateKind},
    fixtures::role::a_minimal_role,
};

fn role_ref(id: &str) -> EntityRef<Role> {
    EntityRef::new(id)
}

#[rstest]
#[case::in_memory(SubstrateKind::InMemory)]
#[case::repo(SubstrateKind::Repo)]
#[tokio::test]
async fn undo_checkout_discards_changes_and_releases_lock(#[case] kind: SubstrateKind) {
    run_with(kind, |workspace| async move {
        workspace.insert(a_minimal_role("eng-lead")).await.unwrap();
        workspace.persist().await.unwrap();

        // Stage a change, then abandon it.
        let mut role = workspace.checkout(role_ref("eng-lead")).await.unwrap();
        role.set_name("Abandoned Change".to_string()).await.unwrap();
        role.undo_checkout().await.unwrap();

        // The change is gone — resolve sees the original.
        let role = workspace.resolve(role_ref("eng-lead")).await.unwrap();
        assert_eq!(role.name().await.unwrap(), "Minimal Role");

        // The lock released — a fresh checkout succeeds and can commit.
        let mut role = workspace.checkout(role_ref("eng-lead")).await.unwrap();
        role.set_name("Real Change".to_string()).await.unwrap();
        role.commit().await.unwrap();
        workspace.persist().await.unwrap();

        let role = workspace.resolve(role_ref("eng-lead")).await.unwrap();
        assert_eq!(role.name().await.unwrap(), "Real Change");
    })
    .await;
}
