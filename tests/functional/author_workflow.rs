//! User job: author a top-level workflow.
//!
//! A workflow plus its embedded entities (a task, optionally a review
//! step) is authored, persisted, and observable to a fresh session.
//! Cross-entity validation runs at insert, so prerequisites (roles,
//! artifact kinds, tasks) are inserted before the workflow itself.

use pari::{
    entity::{AnyEntityRef, EntityRef, TrackedEntity},
    substrate::RepoSubstrate,
    workspace::EntityClient,
};
use rstest::rstest;
use tempfile::TempDir;

use crate::{
    common::substrate::{run_with, SubstrateKind},
    fixtures::{
        artifact_kind::a_minimal_artifact_kind,
        role::a_minimal_role,
        task::a_minimal_task,
        workflow::{a_workflow_with_task_and_review, a_workflow_with_task_step},
    },
};

fn workflow_ref(id: &str) -> AnyEntityRef {
    AnyEntityRef::Workflow(EntityRef::new(id))
}

#[rstest]
#[case::in_memory(SubstrateKind::InMemory)]
#[case::repo(SubstrateKind::Repo)]
#[tokio::test]
async fn minimal_workflow_is_observable_after_persist(#[case] kind: SubstrateKind) {
    run_with(kind, || async {
        EntityClient::insert(a_minimal_role("eng-lead"))
            .await
            .unwrap();
        EntityClient::insert(a_minimal_artifact_kind("design-doc"))
            .await
            .unwrap();
        EntityClient::insert(a_minimal_task("Design", "DesignFlow", "design-doc"))
            .await
            .unwrap();
        EntityClient::insert(a_workflow_with_task_step(
            "DesignFlow",
            "eng-lead",
            "Design",
        ))
        .await
        .unwrap();
        EntityClient::persist().await.unwrap();

        let entity = EntityClient::resolve(workflow_ref("DesignFlow"))
            .await
            .unwrap();
        let TrackedEntity::Workflow(wf) = entity else {
            panic!("expected Workflow")
        };
        assert_eq!(wf.name().await.unwrap(), "Design Workflow");
        assert_eq!(
            wf.purpose().await.unwrap(),
            "Drive a single design through review."
        );
        let raci = wf.raci().await.unwrap().clone();
        assert_eq!(raci.accountable.id(), "eng-lead");
        let states = wf.states().await.unwrap().to_vec();
        assert_eq!(states.len(), 2);
        let steps = wf.steps().await.unwrap().clone();
        assert_eq!(steps.len(), 1);
        assert!(steps.contains_key("Design"));
    })
    .await;
}

#[rstest]
#[case::in_memory(SubstrateKind::InMemory)]
#[case::repo(SubstrateKind::Repo)]
#[tokio::test]
async fn workflow_with_review_step_is_observable_after_persist(#[case] kind: SubstrateKind) {
    run_with(kind, || async {
        EntityClient::insert(a_minimal_role("eng-lead"))
            .await
            .unwrap();
        EntityClient::insert(a_minimal_role("approver"))
            .await
            .unwrap();
        EntityClient::insert(a_minimal_artifact_kind("design-doc"))
            .await
            .unwrap();
        EntityClient::insert(a_minimal_task("Design", "DesignFlow", "design-doc"))
            .await
            .unwrap();
        EntityClient::insert(a_workflow_with_task_and_review(
            "DesignFlow",
            "eng-lead",
            "Design",
            "approver",
        ))
        .await
        .unwrap();
        EntityClient::persist().await.unwrap();

        let entity = EntityClient::resolve(workflow_ref("DesignFlow"))
            .await
            .unwrap();
        let TrackedEntity::Workflow(wf) = entity else {
            panic!("expected Workflow")
        };
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

/// Full workflow with task + review steps cold-starts a fresh
/// [`RepoSubstrate`] over the same on-disk directory.
#[tokio::test]
async fn workflow_round_trips_repo_substrate_across_sessions() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().to_path_buf();

    pari::with(RepoSubstrate::new(path.clone()).unwrap(), || async {
        EntityClient::insert(a_minimal_role("eng-lead"))
            .await
            .unwrap();
        EntityClient::insert(a_minimal_role("approver"))
            .await
            .unwrap();
        EntityClient::insert(a_minimal_artifact_kind("design-doc"))
            .await
            .unwrap();
        EntityClient::insert(a_minimal_task("Design", "DesignFlow", "design-doc"))
            .await
            .unwrap();
        EntityClient::insert(a_workflow_with_task_and_review(
            "DesignFlow",
            "eng-lead",
            "Design",
            "approver",
        ))
        .await
        .unwrap();
        EntityClient::persist().await.unwrap();
    })
    .await;

    pari::with(RepoSubstrate::new(path.clone()).unwrap(), || async {
        let entity = EntityClient::resolve(workflow_ref("DesignFlow"))
            .await
            .unwrap();
        let TrackedEntity::Workflow(wf) = entity else {
            panic!("expected Workflow")
        };
        assert_eq!(wf.name().await.unwrap(), "Design Workflow");
        let steps = wf.steps().await.unwrap().clone();
        assert_eq!(steps.len(), 2);
        assert!(steps.contains_key("Design"));
        assert!(steps.contains_key("Review"));
    })
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

    pari::with(RepoSubstrate::new(path.clone()).unwrap(), || async {
        EntityClient::insert(a_minimal_role("eng-lead"))
            .await
            .unwrap();
        EntityClient::insert(a_minimal_artifact_kind("design-doc"))
            .await
            .unwrap();
        EntityClient::insert(a_minimal_task("Design", "DesignFlow", "design-doc"))
            .await
            .unwrap();
        EntityClient::insert(a_workflow_with_task_step(
            "DesignFlow",
            "eng-lead",
            "Design",
        ))
        .await
        .unwrap();
        EntityClient::persist().await.unwrap();
    })
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
