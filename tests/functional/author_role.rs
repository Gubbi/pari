//! User job: author a role.
//!
//! A new role is defined, persisted, and observable to a fresh session.
//! Substrate-incidental scenarios run against both shipped backends;
//! `RepoSubstrate`-specific scenarios pin the on-disk file shape that
//! external readers depend on.

use pari::{
    entity::{AnyEntityRef, EntityRef, TrackedEntity},
    substrate::RepoSubstrate,
    workspace::EntityClient,
};
use rstest::rstest;
use tempfile::TempDir;

use crate::{
    common::substrate::{run_with, SubstrateKind},
    fixtures::role::{a_minimal_role, a_role_with_optional_fields},
};

fn role_ref(id: &str) -> AnyEntityRef {
    AnyEntityRef::Role(EntityRef::new(id))
}

#[rstest]
#[case::in_memory(SubstrateKind::InMemory)]
#[case::repo(SubstrateKind::Repo)]
#[tokio::test]
async fn minimal_role_is_observable_after_persist(#[case] kind: SubstrateKind) {
    run_with(kind, || async {
        EntityClient::insert(a_minimal_role("eng-lead"))
            .await
            .unwrap();
        EntityClient::persist().await.unwrap();

        let entity = EntityClient::resolve(role_ref("eng-lead")).await.unwrap();
        let TrackedEntity::Role(role) = entity else {
            panic!("expected Role")
        };
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
    run_with(kind, || async {
        EntityClient::insert(a_role_with_optional_fields("eng-lead"))
            .await
            .unwrap();
        EntityClient::persist().await.unwrap();

        let entity = EntityClient::resolve(role_ref("eng-lead")).await.unwrap();
        let TrackedEntity::Role(role) = entity else {
            panic!("expected Role")
        };
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

/// Persisting a role authored against `RepoSubstrate` survives a full
/// session restart over the same on-disk directory.
#[tokio::test]
async fn role_round_trips_repo_substrate_across_sessions() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().to_path_buf();

    pari::with(RepoSubstrate::new(path.clone()).unwrap(), || async {
        EntityClient::insert(a_role_with_optional_fields("eng-lead"))
            .await
            .unwrap();
        EntityClient::persist().await.unwrap();
    })
    .await;

    pari::with(RepoSubstrate::new(path.clone()).unwrap(), || async {
        let entity = EntityClient::resolve(role_ref("eng-lead")).await.unwrap();
        let TrackedEntity::Role(role) = entity else {
            panic!("expected Role")
        };
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
    })
    .await;
}

/// `RepoSubstrate`'s file format is a public output: external tools read
/// `roles/<id>.md` directly. Pin the shape so format changes are
/// deliberate.
#[tokio::test]
async fn repo_substrate_writes_expected_role_file() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().to_path_buf();
    let role_file = path.join("roles/eng-lead.md");

    pari::with(RepoSubstrate::new(path.clone()).unwrap(), || async {
        EntityClient::insert(a_role_with_optional_fields("eng-lead"))
            .await
            .unwrap();
        EntityClient::persist().await.unwrap();
    })
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
