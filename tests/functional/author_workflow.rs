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
    entities::{task::Task, workflow::Workflow},
    entity::{EntityRef, WorkflowParent},
    substrate::RepoSubstrate,
    workspace::Workspace,
};
use rstest::rstest;
use serde_json::json;
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

/// `Task` carries two flatten slots: `FrontmatterFlattened(Prefix("x-"))`
/// for general extensions and `SectionFlattened(Prefix("x-doc-"), …)`
/// for long-form documentation extensions. `x-doc-`-prefixed wire keys
/// must route to markdown sections (longest-prefix-match wins over
/// `x-`), and the on-disk shape round-trips back to bare keys at the
/// workspace boundary.
#[rstest]
#[case::in_memory(SubstrateKind::InMemory)]
#[case::repo(SubstrateKind::Repo)]
#[tokio::test]
async fn task_extensions_route_by_prefix_match(#[case] kind: SubstrateKind) {
    run_with(kind, |workspace| async move {
        workspace.insert(a_minimal_role("eng-lead")).await.unwrap();
        workspace
            .insert(a_minimal_artifact_kind("design-doc"))
            .await
            .unwrap();
        workspace
            .insert(a_workflow_with_empty_steps("DesignFlow", "eng-lead"))
            .await
            .unwrap();

        let mut task = a_minimal_task("Design", "DesignFlow", "design-doc");
        task.extensions.0.insert("color".to_string(), json!("red"));
        task.extensions.0.insert(
            "doc-rationale".to_string(),
            json!("Why this task exists.\n\nLong-form text body."),
        );
        workspace.insert(task).await.unwrap();
        workspace.persist().await.unwrap();

        let task_ref = EntityRef::<Task, _>::with_parent(
            "Design",
            WorkflowParent::Workflow(EntityRef::<Workflow>::new("DesignFlow")),
        );
        workspace.forget(task_ref.clone()).await.unwrap();

        let task = workspace.resolve(task_ref).await.unwrap();
        let extensions = task.extensions().await.unwrap();
        assert_eq!(extensions.0.get("color"), Some(&json!("red")));
        assert_eq!(
            extensions.0.get("doc-rationale"),
            Some(&json!("Why this task exists.\n\nLong-form text body."))
        );
        assert!(
            !extensions.0.keys().any(|k| k.starts_with("x-")),
            "in-memory bag must not retain x- prefix, got: {:?}",
            extensions.0
        );
    })
    .await;
}

/// `RepoSubstrate`-specific: `x-doc-` extensions render as markdown
/// sections, plain `x-` extensions stay in frontmatter. Pins the
/// on-disk shape so external tools can rely on it.
#[tokio::test]
async fn repo_substrate_writes_x_doc_extension_to_section() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().to_path_buf();
    let task_file = path.join("workflows/DesignFlow/Design/README.md");

    with_workspace(
        RepoSubstrate::new(path.clone()).unwrap(),
        |workspace| async move {
            workspace.insert(a_minimal_role("eng-lead")).await.unwrap();
            workspace
                .insert(a_minimal_artifact_kind("design-doc"))
                .await
                .unwrap();
            workspace
                .insert(a_workflow_with_empty_steps("DesignFlow", "eng-lead"))
                .await
                .unwrap();

            let mut task = a_minimal_task("Design", "DesignFlow", "design-doc");
            task.extensions.0.insert("color".to_string(), json!("red"));
            task.extensions
                .0
                .insert("doc-rationale".to_string(), json!("Why this task exists."));
            workspace.insert(task).await.unwrap();
            workspace.persist().await.unwrap();
        },
    )
    .await;

    let raw = std::fs::read_to_string(&task_file).unwrap();
    assert!(
        raw.contains("x-color: red"),
        "expected x-color in frontmatter, got:\n{raw}"
    );
    assert!(
        raw.contains("## x-doc-rationale"),
        "expected x-doc-rationale section heading, got:\n{raw}"
    );
    assert!(
        raw.contains("Why this task exists."),
        "expected section body text, got:\n{raw}"
    );
    assert!(
        !raw.contains("x-doc-rationale: "),
        "x-doc-rationale must not appear as a frontmatter key, got:\n{raw}"
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
