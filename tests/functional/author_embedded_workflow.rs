//! User job: author a workflow whose steps include a nested
//! [`EmbeddedWorkflow`].
//!
//! The iterative pattern recurses one level: insert parent shell,
//! insert embedded shell (parent now exists), insert task (embedded
//! parent now exists), modify embedded.steps to point at task, modify
//! parent.steps to point at embedded.

use pari::{
    entities::workflow::{EmbeddedWorkflow, Workflow},
    entity::{EntityRef, WorkflowParent},
    substrate::RepoSubstrate,
    workspace::Workspace,
};
use rstest::rstest;
use tempfile::TempDir;

use crate::{
    common::substrate::{run_with, with_workspace, SubstrateKind},
    fixtures::{
        artifact_kind::a_minimal_artifact_kind,
        embedded_workflow::{
            a_minimal_embedded_workflow, embedded_workflow_step, task_step_for_embedded,
        },
        role::a_minimal_role,
        task::a_minimal_task_with_parent,
        workflow::a_workflow_with_empty_steps,
    },
};

fn workflow_ref(id: &str) -> EntityRef<Workflow> {
    EntityRef::new(id)
}

fn embedded_workflow_ref(
    id: &str,
    parent_workflow_id: &str,
) -> EntityRef<EmbeddedWorkflow, WorkflowParent> {
    let parent = WorkflowParent::Workflow(EntityRef::<Workflow>::new(parent_workflow_id));
    EntityRef::with_parent(id, parent)
}

#[rstest]
#[case::in_memory(SubstrateKind::InMemory)]
#[case::repo(SubstrateKind::Repo)]
#[tokio::test]
async fn author_embedded_workflow_with_task(#[case] kind: SubstrateKind) {
    run_with(kind, |workspace| async move {
        author_nested_workflow(&workspace).await;

        let parent_wf = workspace.resolve(workflow_ref("DesignFlow")).await.unwrap();
        let parent_steps = parent_wf.steps().await.unwrap().clone();
        assert_eq!(parent_steps.len(), 1);
        assert!(parent_steps.contains_key("Onboarding"));

        let embedded = workspace
            .resolve(embedded_workflow_ref("Onboarding", "DesignFlow"))
            .await
            .unwrap();
        assert_eq!(embedded.name().await.unwrap(), "Onboarding");
        let embedded_steps = embedded.steps().await.unwrap().clone();
        assert_eq!(embedded_steps.len(), 1);
        assert!(embedded_steps.contains_key("Welcome"));
    })
    .await;
}

#[tokio::test]
async fn embedded_workflow_round_trips_repo_substrate() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().to_path_buf();

    with_workspace(
        RepoSubstrate::new(path.clone()).unwrap(),
        |workspace| async move {
            author_nested_workflow(&workspace).await;
        },
    )
    .await;

    with_workspace(
        RepoSubstrate::new(path.clone()).unwrap(),
        |workspace| async move {
            let parent_wf = workspace.resolve(workflow_ref("DesignFlow")).await.unwrap();
            assert!(parent_wf.steps().await.unwrap().contains_key("Onboarding"));

            let embedded = workspace
                .resolve(embedded_workflow_ref("Onboarding", "DesignFlow"))
                .await
                .unwrap();
            assert!(embedded.steps().await.unwrap().contains_key("Welcome"));
        },
    )
    .await;
}

#[tokio::test]
async fn repo_substrate_writes_expected_embedded_workflow_files() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().to_path_buf();
    let parent_file = path.join("workflows/DesignFlow/README.md");
    let embedded_file = path.join("workflows/DesignFlow/Onboarding/README.md");
    let task_file = path.join("workflows/DesignFlow/Onboarding/Welcome/README.md");

    with_workspace(
        RepoSubstrate::new(path.clone()).unwrap(),
        |workspace| async move {
            author_nested_workflow(&workspace).await;
        },
    )
    .await;

    assert!(parent_file.exists(), "expected {parent_file:?}");
    assert!(embedded_file.exists(), "expected {embedded_file:?}");
    assert!(task_file.exists(), "expected {task_file:?}");

    let embedded_contents = std::fs::read_to_string(&embedded_file).unwrap();
    assert!(
        embedded_contents.contains("# Onboarding"),
        "expected embedded H1, got:\n{embedded_contents}"
    );
}

/// Iterative author flow: prerequisites, parent shell, embedded shell,
/// task under embedded, modify embedded steps, modify parent steps.
async fn author_nested_workflow(workspace: &Workspace) {
    workspace.insert(a_minimal_role("eng-lead")).await.unwrap();
    workspace
        .insert(a_minimal_artifact_kind("design-doc"))
        .await
        .unwrap();
    workspace
        .insert(a_workflow_with_empty_steps("DesignFlow", "eng-lead"))
        .await
        .unwrap();
    workspace
        .insert(a_minimal_embedded_workflow(
            "Onboarding",
            "DesignFlow",
            "eng-lead",
        ))
        .await
        .unwrap();

    let task_parent =
        WorkflowParent::EmbeddedWorkflow(Box::new(EntityRef::<EmbeddedWorkflow, _>::with_parent(
            "Onboarding",
            WorkflowParent::Workflow(EntityRef::<Workflow>::new("DesignFlow")),
        )));
    workspace
        .insert(a_minimal_task_with_parent(
            "Welcome",
            task_parent,
            "design-doc",
        ))
        .await
        .unwrap();

    workspace.persist().await.unwrap();

    // Modify embedded workflow steps to reference the task.
    let embedded_typed = EntityRef::<EmbeddedWorkflow, _>::with_parent(
        "Onboarding",
        WorkflowParent::Workflow(EntityRef::<Workflow>::new("DesignFlow")),
    );
    let mut embedded = workspace.checkout(embedded_typed).await.unwrap();
    embedded
        .set_steps(task_step_for_embedded(
            "Welcome",
            "Onboarding",
            "DesignFlow",
        ))
        .await
        .unwrap();
    embedded.commit().await.unwrap();

    // Modify parent workflow steps to reference the embedded workflow.
    let mut parent = workspace
        .checkout(EntityRef::<Workflow>::new("DesignFlow"))
        .await
        .unwrap();
    parent
        .set_steps(embedded_workflow_step("Onboarding", "DesignFlow"))
        .await
        .unwrap();
    parent.commit().await.unwrap();

    workspace.persist().await.unwrap();
}
