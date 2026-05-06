//! User job: author a top-level workflow.
//!
//! A workflow is authored iteratively to satisfy two cross-entity
//! invariants that hold at every transaction boundary: every embedded
//! entity's parent must exist, and every ref a workflow names in its
//! steps must exist. The pattern: insert a workflow shell with empty
//! steps so the parent is in the substrate; insert each embedded entity
//! (a task here) under it; modify the workflow's `steps` to its final
//! shape.

use pari::{
    entities::workflow::Workflow, entity::EntityRef, substrate::RepoSubstrate, workspace::Workspace,
};
use rstest::rstest;
use tempfile::TempDir;

use crate::{
    common::substrate::{run_with, with_workspace, SubstrateKind},
    fixtures::{
        artifact_kind::a_minimal_artifact_kind,
        role::a_minimal_role,
        task::a_minimal_task,
        workflow::{a_workflow_with_empty_steps, task_and_review_steps},
    },
};

fn workflow_ref(id: &str) -> EntityRef<Workflow> {
    EntityRef::new(id)
}

#[rstest]
#[case::in_memory(SubstrateKind::InMemory)]
#[case::repo(SubstrateKind::Repo)]
#[tokio::test]
async fn workflow_with_task_and_review_is_observable_after_persist(#[case] kind: SubstrateKind) {
    run_with(kind, |workspace| async move {
        author_workflow_with_task_and_review(&workspace).await;

        let wf = workspace.resolve(workflow_ref("DesignFlow")).await.unwrap();
        assert_eq!(wf.name().await.unwrap(), "Design Workflow");
        let steps = wf.steps().await.unwrap().clone();
        assert_eq!(steps.len(), 2);
        assert!(steps.contains_key("Design"));
        assert!(steps.contains_key("Review"));
        let states = wf.states().await.unwrap().to_vec();
        assert_eq!(states.len(), 3);
        assert!(states.iter().any(|s| s.id == "InReview"));
    })
    .await;
}

#[tokio::test]
async fn workflow_round_trips_repo_substrate_across_sessions() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().to_path_buf();

    with_workspace(
        RepoSubstrate::new(path.clone()).unwrap(),
        |workspace| async move {
            author_workflow_with_task_and_review(&workspace).await;
        },
    )
    .await;

    with_workspace(
        RepoSubstrate::new(path.clone()).unwrap(),
        |workspace| async move {
            let wf = workspace.resolve(workflow_ref("DesignFlow")).await.unwrap();
            assert_eq!(wf.name().await.unwrap(), "Design Workflow");
            let steps = wf.steps().await.unwrap().clone();
            assert_eq!(steps.len(), 2);
            assert!(steps.contains_key("Design"));
            assert!(steps.contains_key("Review"));
        },
    )
    .await;
}

/// `RepoSubstrate` writes the workflow's README and the embedded task's
/// README under the parent workflow directory — both consumed by
/// external tools.
#[tokio::test]
async fn repo_substrate_writes_expected_workflow_files() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().to_path_buf();
    let workflow_file = path.join("workflows/DesignFlow/README.md");
    let task_file = path.join("workflows/DesignFlow/Design/README.md");

    with_workspace(
        RepoSubstrate::new(path.clone()).unwrap(),
        |workspace| async move {
            author_workflow_with_task_and_review(&workspace).await;
        },
    )
    .await;

    assert!(workflow_file.exists(), "expected {workflow_file:?}");
    assert!(task_file.exists(), "expected {task_file:?}");

    let wf_contents = std::fs::read_to_string(&workflow_file).unwrap();
    assert!(
        wf_contents.contains("# Design Workflow"),
        "expected H1 with workflow name, got:\n{wf_contents}"
    );
    assert!(
        wf_contents.contains("steps:"),
        "expected steps frontmatter key, got:\n{wf_contents}"
    );

    let task_contents = std::fs::read_to_string(&task_file).unwrap();
    assert!(
        task_contents.contains("# Design Doc Draft"),
        "expected H1 with task name, got:\n{task_contents}"
    );
}

/// Iterative author flow used by every scenario above: prerequisites,
/// workflow shell with empty steps, the embedded task (its parent now
/// exists), then modify steps to the final shape (task + review).
/// Returns after persist completes.
async fn author_workflow_with_task_and_review(workspace: &Workspace) {
    workspace.insert(a_minimal_role("eng-lead")).await.unwrap();
    workspace.insert(a_minimal_role("approver")).await.unwrap();
    workspace
        .insert(a_minimal_artifact_kind("design-doc"))
        .await
        .unwrap();

    workspace
        .insert(a_workflow_with_empty_steps("DesignFlow", "eng-lead"))
        .await
        .unwrap();
    workspace.persist().await.unwrap();

    workspace
        .insert(a_minimal_task("Design", "DesignFlow", "design-doc"))
        .await
        .unwrap();

    let mut wf = workspace
        .checkout(EntityRef::<Workflow>::new("DesignFlow"))
        .await
        .unwrap();
    wf.set_steps(task_and_review_steps("Design", "DesignFlow", "approver"))
        .await
        .unwrap();
    wf.commit().await.unwrap();
    workspace.persist().await.unwrap();
}
