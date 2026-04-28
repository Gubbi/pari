//! User job: author a workflow whose steps include a [`Relay`].
//!
//! A relay is an embedded entity (parent = workflow) that delegates to
//! a [`ReusableWorkflow`]. The iterative pattern: insert prerequisites
//! (roles + reusable workflow + workflow shell) → insert relay (its
//! parent now exists) → modify the workflow's steps to include
//! `Step::Relay`.

use pari::{
    entities::workflow::Workflow,
    entity::{AnyEntityRef, EntityRef, TrackedEntity, WorkflowParent},
    substrate::RepoSubstrate,
    workspace::EntityClient,
};
use rstest::rstest;
use tempfile::TempDir;

use crate::{
    common::substrate::{run_with, SubstrateKind},
    fixtures::{
        relay::{a_minimal_relay, relay_step},
        reusable_workflow::a_reusable_workflow_with_review_step,
        role::a_minimal_role,
        workflow::a_workflow_with_empty_steps,
    },
};

fn workflow_ref(id: &str) -> AnyEntityRef {
    AnyEntityRef::Workflow(EntityRef::new(id))
}

fn relay_ref(id: &str, parent_workflow_id: &str) -> AnyEntityRef {
    let parent = WorkflowParent::Workflow(EntityRef::<Workflow>::new(parent_workflow_id));
    AnyEntityRef::Relay(EntityRef::with_parent(id, parent))
}

#[rstest]
#[case::in_memory(SubstrateKind::InMemory)]
#[case::repo(SubstrateKind::Repo)]
#[tokio::test]
async fn author_relay_in_workflow(#[case] kind: SubstrateKind) {
    run_with(kind, || async {
        author_workflow_with_relay().await;

        let entity = EntityClient::resolve(workflow_ref("DesignFlow"))
            .await
            .unwrap();
        let TrackedEntity::Workflow(wf) = entity else {
            panic!("expected Workflow")
        };
        let steps = wf.steps().await.unwrap().clone();
        assert_eq!(steps.len(), 1);
        assert!(steps.contains_key("Handoff"));

        let relay_entity = EntityClient::resolve(relay_ref("Handoff", "DesignFlow"))
            .await
            .unwrap();
        let TrackedEntity::Relay(relay) = relay_entity else {
            panic!("expected Relay")
        };
        assert_eq!(relay.delegates_to().await.unwrap().id(), "ApprovalLoop");
    })
    .await;
}

#[tokio::test]
async fn relay_round_trips_repo_substrate() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().to_path_buf();

    pari::with(RepoSubstrate::new(path.clone()).unwrap(), || async {
        author_workflow_with_relay().await;
    })
    .await;

    pari::with(RepoSubstrate::new(path.clone()).unwrap(), || async {
        let entity = EntityClient::resolve(workflow_ref("DesignFlow"))
            .await
            .unwrap();
        let TrackedEntity::Workflow(wf) = entity else {
            panic!("expected Workflow")
        };
        assert!(wf.steps().await.unwrap().contains_key("Handoff"));

        let relay_entity = EntityClient::resolve(relay_ref("Handoff", "DesignFlow"))
            .await
            .unwrap();
        let TrackedEntity::Relay(relay) = relay_entity else {
            panic!("expected Relay")
        };
        assert_eq!(relay.delegates_to().await.unwrap().id(), "ApprovalLoop");
    })
    .await;
}

#[tokio::test]
async fn repo_substrate_writes_expected_relay_files() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().to_path_buf();
    let workflow_file = path.join("workflows/DesignFlow/README.md");
    let relay_file = path.join("workflows/DesignFlow/Handoff/README.md");

    pari::with(RepoSubstrate::new(path.clone()).unwrap(), || async {
        author_workflow_with_relay().await;
    })
    .await;

    assert!(workflow_file.exists(), "expected {workflow_file:?}");
    assert!(relay_file.exists(), "expected {relay_file:?}");

    let relay_contents = std::fs::read_to_string(&relay_file).unwrap();
    assert!(
        relay_contents.contains("# Approval Handoff"),
        "expected relay H1, got:\n{relay_contents}"
    );
    assert!(
        relay_contents.contains("delegates_to:"),
        "expected delegates_to frontmatter key, got:\n{relay_contents}"
    );
}

/// Iterative author flow: prerequisites, reusable workflow, workflow
/// shell, relay (parent now exists), modify workflow steps to include
/// the relay.
async fn author_workflow_with_relay() {
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
    EntityClient::insert(a_workflow_with_empty_steps("DesignFlow", "eng-lead"))
        .await
        .unwrap();
    EntityClient::insert(a_minimal_relay(
        "Handoff",
        "DesignFlow",
        "eng-lead",
        "ApprovalLoop",
    ))
    .await
    .unwrap();
    EntityClient::persist().await.unwrap();

    let mut entity = EntityClient::checkout(workflow_ref("DesignFlow"))
        .await
        .unwrap();
    if let TrackedEntity::Workflow(ref mut wf) = entity {
        wf.set_steps(relay_step("Handoff", "DesignFlow"))
            .await
            .unwrap();
    }
    entity.commit().await.unwrap();
    EntityClient::persist().await.unwrap();
}
