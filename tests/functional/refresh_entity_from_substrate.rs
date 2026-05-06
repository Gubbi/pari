//! User job: refresh an entity from the substrate.
//!
//! `forget` drops a clean entity's loaded fields back to a stub. A
//! subsequent accessor refetches from the substrate. The point of the
//! operation is to pick up changes made externally to the substrate
//! without restarting the session.

use pari::{entities::role::Role, entity::EntityRef, substrate::RepoSubstrate};
use rstest::rstest;
use tempfile::TempDir;

use crate::{
    common::substrate::{run_with, with_workspace, SubstrateKind},
    fixtures::role::a_minimal_role,
};

fn role_ref(id: &str) -> EntityRef<Role> {
    EntityRef::new(id)
}

#[rstest]
#[case::in_memory(SubstrateKind::InMemory)]
#[case::repo(SubstrateKind::Repo)]
#[tokio::test]
async fn forget_clean_entity_succeeds_and_reload_works(#[case] kind: SubstrateKind) {
    run_with(kind, |workspace| async move {
        workspace.insert(a_minimal_role("eng-lead")).await.unwrap();
        workspace.persist().await.unwrap();

        // First access: triggers the initial load (or hits the
        // already-loaded in-store entity).
        let role = workspace.resolve(role_ref("eng-lead")).await.unwrap();
        assert_eq!(role.name().await.unwrap(), "Minimal Role");

        workspace.forget(role_ref("eng-lead")).await.unwrap();

        // Second access: stub re-fetches transparently.
        let role = workspace.resolve(role_ref("eng-lead")).await.unwrap();
        assert_eq!(role.name().await.unwrap(), "Minimal Role");
    })
    .await;
}

/// External edits to a `RepoSubstrate` file are picked up by `forget`
/// + accessor refetch.
#[tokio::test]
async fn forget_picks_up_external_substrate_change() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().to_path_buf();
    let role_file = path.join("common/roles/eng-lead.md");

    with_workspace(
        RepoSubstrate::new(path.clone()).unwrap(),
        |workspace| async move {
            workspace.insert(a_minimal_role("eng-lead")).await.unwrap();
            workspace.persist().await.unwrap();

            let role = workspace.resolve(role_ref("eng-lead")).await.unwrap();
            assert_eq!(role.name().await.unwrap(), "Minimal Role");

            // Edit the file externally — replace the H1 (which the codec
            // maps to `name`).
            let original = std::fs::read_to_string(&role_file).unwrap();
            let edited = original.replace("# Minimal Role", "# Externally Edited");
            std::fs::write(&role_file, edited).unwrap();

            workspace.forget(role_ref("eng-lead")).await.unwrap();

            // Resolve + accessor now refetches from the (modified) file.
            let role = workspace.resolve(role_ref("eng-lead")).await.unwrap();
            assert_eq!(role.name().await.unwrap(), "Externally Edited");
        },
    )
    .await;
}
