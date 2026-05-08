//! User job: author a standalone reusable workflow.
//!
//! A reusable workflow is a library definition other workflows delegate
//! to via [`Relay`]. It is top-level (no parent), so authoring is
//! single-shot — no chicken-and-egg with embedded children.

use pari::{
    entities::workflow::ReusableWorkflow, entity::EntityRef, substrate::RepoSubstrate,
    workspace::Workspace,
};
use rstest::rstest;
use tempfile::TempDir;

use crate::{
    common::substrate::{run_with, with_workspace, SubstrateKind},
    fixtures::{reusable_workflow::a_reusable_workflow_with_review_step, role::a_minimal_role},
};

fn reusable_workflow_ref(id: &str) -> EntityRef<ReusableWorkflow> {
    EntityRef::new(id)
}

#[rstest]
#[case::in_memory(SubstrateKind::InMemory)]
#[case::repo(SubstrateKind::Repo)]
#[tokio::test]
async fn minimal_reusable_workflow_is_observable_after_persist(#[case] kind: SubstrateKind) {
    run_with(kind, |workspace| async move {
        author_minimal_reusable_workflow(&workspace).await;

        // Drop loaded fields so the accessors below drive the codec +
        // schema gate on load.
        workspace
            .forget(reusable_workflow_ref("ApprovalLoop"))
            .await
            .unwrap();

        let rwf = workspace
            .resolve(reusable_workflow_ref("ApprovalLoop"))
            .await
            .unwrap();
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

    with_workspace(
        RepoSubstrate::new(path.clone()).unwrap(),
        |workspace| async move {
            author_minimal_reusable_workflow(&workspace).await;
        },
    )
    .await;

    with_workspace(
        RepoSubstrate::new(path.clone()).unwrap(),
        |workspace| async move {
            let rwf = workspace
                .resolve(reusable_workflow_ref("ApprovalLoop"))
                .await
                .unwrap();
            assert_eq!(rwf.name().await.unwrap(), "Approval Loop");
        },
    )
    .await;
}

#[tokio::test]
async fn repo_substrate_writes_expected_reusable_workflow_files() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().to_path_buf();
    let rwf_file = path.join("common/workflows/ApprovalLoop/README.md");

    with_workspace(
        RepoSubstrate::new(path.clone()).unwrap(),
        |workspace| async move {
            author_minimal_reusable_workflow(&workspace).await;
        },
    )
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

async fn author_minimal_reusable_workflow(workspace: &Workspace) {
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
    workspace.persist().await.unwrap();
}
