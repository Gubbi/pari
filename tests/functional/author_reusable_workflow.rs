//! User job: author a standalone reusable workflow.
//!
//! A reusable workflow is a library definition other workflows delegate
//! to via [`Relay`]. It is top-level (no parent), so authoring is
//! single-shot — no chicken-and-egg with embedded children.

use pari::{
    entity::{AnyEntityRef, EntityRef, TrackedEntity},
    substrate::RepoSubstrate,
    workspace::EntityClient,
};
use rstest::rstest;
use tempfile::TempDir;

use crate::{
    common::substrate::{run_with, SubstrateKind},
    fixtures::{reusable_workflow::a_reusable_workflow_with_review_step, role::a_minimal_role},
};

fn reusable_workflow_ref(id: &str) -> AnyEntityRef {
    AnyEntityRef::ReusableWorkflow(EntityRef::new(id))
}

#[rstest]
#[case::in_memory(SubstrateKind::InMemory)]
#[case::repo(SubstrateKind::Repo)]
#[tokio::test]
async fn minimal_reusable_workflow_is_observable_after_persist(#[case] kind: SubstrateKind) {
    run_with(kind, || async {
        author_minimal_reusable_workflow().await;

        let entity = EntityClient::resolve(reusable_workflow_ref("ApprovalLoop"))
            .await
            .unwrap();
        let TrackedEntity::ReusableWorkflow(rwf) = entity else {
            panic!("expected ReusableWorkflow")
        };
        assert_eq!(rwf.name().await.unwrap(), "Approval Loop");
        let steps = rwf.steps().await.unwrap().clone();
        assert_eq!(steps.len(), 1);
        assert!(steps.contains_key("Review"));
        let states = rwf.states().await.unwrap().to_vec();
        assert_eq!(states.len(), 3);
        assert!(states.iter().any(|s| s.id == "Reviewing"));
    })
    .await;
}

#[tokio::test]
async fn reusable_workflow_round_trips_repo_substrate() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().to_path_buf();

    pari::with(RepoSubstrate::new(path.clone()).unwrap(), || async {
        author_minimal_reusable_workflow().await;
    })
    .await;

    pari::with(RepoSubstrate::new(path.clone()).unwrap(), || async {
        let entity = EntityClient::resolve(reusable_workflow_ref("ApprovalLoop"))
            .await
            .unwrap();
        let TrackedEntity::ReusableWorkflow(rwf) = entity else {
            panic!("expected ReusableWorkflow")
        };
        assert_eq!(rwf.name().await.unwrap(), "Approval Loop");
    })
    .await;
}

#[tokio::test]
async fn repo_substrate_writes_expected_reusable_workflow_files() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().to_path_buf();
    let rwf_file = path.join("reusable-workflows/ApprovalLoop/README.md");

    pari::with(RepoSubstrate::new(path.clone()).unwrap(), || async {
        author_minimal_reusable_workflow().await;
    })
    .await;

    assert!(rwf_file.exists(), "expected {rwf_file:?}");
    let contents = std::fs::read_to_string(&rwf_file).unwrap();
    assert!(
        contents.contains("# Approval Loop"),
        "expected H1 with reusable workflow name, got:\n{contents}"
    );
    assert!(
        contents.contains("steps:"),
        "expected steps frontmatter key, got:\n{contents}"
    );
}

async fn author_minimal_reusable_workflow() {
    EntityClient::insert(a_minimal_role("eng-lead"))
        .await
        .unwrap();
    EntityClient::insert(a_minimal_role("approver"))
        .await
        .unwrap();
    EntityClient::insert(a_reusable_workflow_with_review_step(
        "ApprovalLoop",
        "eng-lead",
        "approver",
    ))
    .await
    .unwrap();
    EntityClient::persist().await.unwrap();
}
