//! User job: author a role.
//!
//! A new role is defined, persisted, and observable to a fresh session.
//! Substrate-incidental scenarios run against both shipped backends;
//! `RepoSubstrate`-specific scenarios pin the on-disk file shape that
//! external readers depend on.

use pari::{entities::role::Role, entity::EntityRef, substrate::RepoSubstrate};
use rstest::rstest;
use serde_json::json;
use tempfile::TempDir;

use crate::{
    common::substrate::{run_with, with_workspace, SubstrateKind},
    fixtures::role::{a_minimal_role, a_role_with_optional_fields},
};

fn role_ref(id: &str) -> EntityRef<Role> {
    EntityRef::new(id)
}

#[rstest]
#[case::in_memory(SubstrateKind::InMemory)]
#[case::repo(SubstrateKind::Repo)]
#[tokio::test]
async fn minimal_role_is_observable_after_persist(#[case] kind: SubstrateKind) {
    run_with(kind, |workspace| async move {
        workspace.insert(a_minimal_role("eng-lead")).await.unwrap();
        workspace.persist().await.unwrap();

        let role = workspace.resolve(role_ref("eng-lead")).await.unwrap();
        assert_eq!(role.name().await.unwrap(), "Minimal Role");
        assert_eq!(role.purpose().await.unwrap(), "test purpose");
        assert_eq!(role.description().await.unwrap(), None);
        assert_eq!(role.traits().await.unwrap(), None);
    })
    .await;
}

#[rstest]
#[case::in_memory(SubstrateKind::InMemory)]
#[case::repo(SubstrateKind::Repo)]
#[tokio::test]
async fn role_with_optional_fields_is_observable_after_persist(#[case] kind: SubstrateKind) {
    run_with(kind, |workspace| async move {
        workspace
            .insert(a_role_with_optional_fields("eng-lead"))
            .await
            .unwrap();
        workspace.persist().await.unwrap();

        let role = workspace.resolve(role_ref("eng-lead")).await.unwrap();
        assert_eq!(role.name().await.unwrap(), "Engineering Lead");
        assert_eq!(
            role.description().await.unwrap(),
            Some("Owns delivery of the engineering roadmap.")
        );
        assert_eq!(
            role.traits().await.unwrap(),
            Some(["accountable".to_string(), "technical".to_string()].as_slice())
        );
    })
    .await;
}

/// Populated `Extensions` round-trip through `insert + persist +
/// forget + resolve` against both substrates. Covers the codec/slot
/// refactor: in-memory keeps `x-` keys flat in its stored blob, repo
/// renders them into frontmatter; both surfaces strip the `x-` prefix
/// back to bare keys at the workspace boundary.
#[rstest]
#[case::in_memory(SubstrateKind::InMemory)]
#[case::repo(SubstrateKind::Repo)]
#[tokio::test]
async fn role_with_extensions_round_trips_through_persist(#[case] kind: SubstrateKind) {
    run_with(kind, |workspace| async move {
        let mut role = a_minimal_role("eng-lead");
        role.extensions.0.insert("color".to_string(), json!("red"));
        role.extensions.0.insert("priority".to_string(), json!(7));
        role.extensions
            .0
            .insert("nested".to_string(), json!({"deep": [1, 2, 3]}));
        workspace.insert(role).await.unwrap();
        workspace.persist().await.unwrap();

        // Drop the loaded fields so the next access has to refetch
        // through the codec.
        workspace.forget(role_ref("eng-lead")).await.unwrap();

        let role = workspace.resolve(role_ref("eng-lead")).await.unwrap();
        let extensions = role.extensions().await.unwrap();
        assert_eq!(extensions.0.get("color"), Some(&json!("red")));
        assert_eq!(extensions.0.get("priority"), Some(&json!(7)));
        assert_eq!(
            extensions.0.get("nested"),
            Some(&json!({"deep": [1, 2, 3]}))
        );
        assert!(
            !extensions.0.keys().any(|k| k.starts_with("x-")),
            "in-memory bag must not retain x- prefix, got: {:?}",
            extensions.0
        );
    })
    .await;
}

/// Persisting a role authored against `RepoSubstrate` survives a full
/// session restart over the same on-disk directory.
#[tokio::test]
async fn role_round_trips_repo_substrate_across_sessions() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().to_path_buf();

    with_workspace(
        RepoSubstrate::new(path.clone()).unwrap(),
        |workspace| async move {
            workspace
                .insert(a_role_with_optional_fields("eng-lead"))
                .await
                .unwrap();
            workspace.persist().await.unwrap();
        },
    )
    .await;

    with_workspace(
        RepoSubstrate::new(path.clone()).unwrap(),
        |workspace| async move {
            let role = workspace.resolve(role_ref("eng-lead")).await.unwrap();
            assert_eq!(role.name().await.unwrap(), "Engineering Lead");
            assert_eq!(
                role.description().await.unwrap(),
                Some("Owns delivery of the engineering roadmap.")
            );
            assert_eq!(role.purpose().await.unwrap(), "test purpose");
            assert_eq!(
                role.traits().await.unwrap(),
                Some(["accountable".to_string(), "technical".to_string()].as_slice())
            );
        },
    )
    .await;
}

/// `RepoSubstrate`'s file format is a public output: external tools read
/// `common/roles/<id>.md` directly. Pin the shape so format changes are
/// deliberate.
#[tokio::test]
async fn repo_substrate_writes_expected_role_file() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().to_path_buf();
    let role_file = path.join("common/roles/eng-lead.md");

    with_workspace(
        RepoSubstrate::new(path.clone()).unwrap(),
        |workspace| async move {
            workspace
                .insert(a_role_with_optional_fields("eng-lead"))
                .await
                .unwrap();
            workspace.persist().await.unwrap();
        },
    )
    .await;

    assert!(role_file.exists(), "expected {role_file:?} to be created");
    let contents = std::fs::read_to_string(&role_file).unwrap();

    assert!(
        contents.contains("# Engineering Lead"),
        "expected H1 with role name, got:\n{contents}"
    );
    assert!(
        contents.contains("Owns delivery of the engineering roadmap."),
        "expected description paragraph, got:\n{contents}"
    );
    assert!(
        contents.contains("purpose:"),
        "expected purpose frontmatter key, got:\n{contents}"
    );
    assert!(
        contents.contains("test purpose"),
        "expected purpose value, got:\n{contents}"
    );
    assert!(
        contents.contains("traits:"),
        "expected traits frontmatter key, got:\n{contents}"
    );
    assert!(
        contents.contains("accountable"),
        "expected accountable trait, got:\n{contents}"
    );
    assert!(
        contents.contains("technical"),
        "expected technical trait, got:\n{contents}"
    );
}
