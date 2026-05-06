//! User job: modify a persisted entity.
//!
//! Take an entity that is already in the substrate (or freshly inserted
//! and persisted), check it out, change a field, commit, persist, and
//! observe the new value via a fresh resolve. For repo substrates the
//! on-disk artifact is part of the public output and is asserted
//! against alongside the API result.

use pari::{
    entities::{relay::Relay, role::Role, team::Team, workflow::Workflow},
    entity::{EntityRef, WorkflowParent},
    substrate::RepoSubstrate,
};
use rstest::rstest;
use tempfile::TempDir;

use crate::{
    common::substrate::{run_with, with_workspace, SubstrateKind},
    fixtures::{
        relay::a_minimal_relay,
        reusable_workflow::a_reusable_workflow_with_review_step,
        role::a_minimal_role,
        team::{a_minimal_team, a_team_with_composition},
        workflow::a_workflow_with_empty_steps,
    },
};

fn role_ref(id: &str) -> EntityRef<Role> {
    EntityRef::new(id)
}

fn relay_ref(id: &str, workflow_id: &str) -> EntityRef<Relay, WorkflowParent> {
    let parent = WorkflowParent::Workflow(EntityRef::<Workflow>::new(workflow_id));
    EntityRef::with_parent(id, parent)
}

fn team_ref(id: &str) -> EntityRef<Team> {
    EntityRef::new(id)
}

#[rstest]
#[case::in_memory(SubstrateKind::InMemory)]
#[case::repo(SubstrateKind::Repo)]
#[tokio::test]
async fn modify_required_field_within_session(#[case] kind: SubstrateKind) {
    run_with(kind, |workspace| async move {
        workspace.insert(a_minimal_role("eng-lead")).await.unwrap();
        workspace.persist().await.unwrap();

        let mut role = workspace.checkout(role_ref("eng-lead")).await.unwrap();
        role.set_name("Engineering Lead".to_string()).await.unwrap();
        role.commit().await.unwrap();
        workspace.persist().await.unwrap();

        let role = workspace.resolve(role_ref("eng-lead")).await.unwrap();
        assert_eq!(role.name().await.unwrap(), "Engineering Lead");
    })
    .await;
}

#[rstest]
#[case::in_memory(SubstrateKind::InMemory)]
#[case::repo(SubstrateKind::Repo)]
#[tokio::test]
async fn modify_optional_field_within_session(#[case] kind: SubstrateKind) {
    run_with(kind, |workspace| async move {
        workspace.insert(a_minimal_role("eng-lead")).await.unwrap();
        workspace.persist().await.unwrap();

        // None -> Some
        let mut role = workspace.checkout(role_ref("eng-lead")).await.unwrap();
        role.set_description(Some("Owns delivery.".to_string()))
            .await
            .unwrap();
        role.commit().await.unwrap();
        workspace.persist().await.unwrap();

        let role = workspace.resolve(role_ref("eng-lead")).await.unwrap();
        assert_eq!(role.description().await.unwrap(), Some("Owns delivery."));

        // Some -> None
        let mut role = workspace.checkout(role_ref("eng-lead")).await.unwrap();
        role.set_description(None).await.unwrap();
        role.commit().await.unwrap();
        workspace.persist().await.unwrap();

        let role = workspace.resolve(role_ref("eng-lead")).await.unwrap();
        assert_eq!(role.description().await.unwrap(), None);
    })
    .await;
}

/// Modification authored in one session is visible to a fresh session
/// over the same on-disk directory, and the on-disk file reflects the
/// new value.
#[tokio::test]
async fn modify_field_across_repo_sessions() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().to_path_buf();
    let role_file = path.join("common/roles/eng-lead.md");

    with_workspace(
        RepoSubstrate::new(path.clone()).unwrap(),
        |workspace| async move {
            workspace.insert(a_minimal_role("eng-lead")).await.unwrap();
            workspace.persist().await.unwrap();
        },
    )
    .await;

    with_workspace(
        RepoSubstrate::new(path.clone()).unwrap(),
        |workspace| async move {
            workspace.resolve(role_ref("eng-lead")).await.unwrap();
            let mut role = workspace.checkout(role_ref("eng-lead")).await.unwrap();
            role.set_name("Engineering Lead".to_string()).await.unwrap();
            role.commit().await.unwrap();
            workspace.persist().await.unwrap();
        },
    )
    .await;

    // Third session: confirm the new value is observable through resolve.
    with_workspace(
        RepoSubstrate::new(path.clone()).unwrap(),
        |workspace| async move {
            let role = workspace.resolve(role_ref("eng-lead")).await.unwrap();
            assert_eq!(role.name().await.unwrap(), "Engineering Lead");
        },
    )
    .await;

    let contents = std::fs::read_to_string(&role_file).unwrap();
    assert!(
        contents.contains("# Engineering Lead"),
        "expected H1 with new role name, got:\n{contents}"
    );
}

#[rstest]
#[case::in_memory(SubstrateKind::InMemory)]
#[case::repo(SubstrateKind::Repo)]
#[tokio::test]
async fn modify_relay_delegates_to(#[case] kind: SubstrateKind) {
    run_with(kind, |workspace| async move {
        workspace.insert(a_minimal_role("eng-lead")).await.unwrap();
        workspace.insert(a_minimal_role("approver")).await.unwrap();

        workspace
            .insert(a_reusable_workflow_with_review_step(
                "ApprovalAlpha",
                "eng-lead",
                "approver",
            ))
            .await
            .unwrap();
        workspace
            .insert(a_reusable_workflow_with_review_step(
                "ApprovalBeta",
                "eng-lead",
                "approver",
            ))
            .await
            .unwrap();

        workspace
            .insert(a_workflow_with_empty_steps("DesignFlow", "eng-lead"))
            .await
            .unwrap();
        workspace
            .insert(a_minimal_relay(
                "Handoff",
                "DesignFlow",
                "eng-lead",
                "ApprovalAlpha",
            ))
            .await
            .unwrap();
        workspace.persist().await.unwrap();

        let mut relay = workspace
            .checkout(relay_ref("Handoff", "DesignFlow"))
            .await
            .unwrap();
        relay
            .set_delegates_to(EntityRef::new("ApprovalBeta"))
            .await
            .unwrap();
        relay.commit().await.unwrap();
        workspace.persist().await.unwrap();

        let relay = workspace
            .resolve(relay_ref("Handoff", "DesignFlow"))
            .await
            .unwrap();
        assert_eq!(relay.delegates_to().await.unwrap().id(), "ApprovalBeta");
    })
    .await;
}

#[rstest]
#[case::in_memory(SubstrateKind::InMemory)]
#[case::repo(SubstrateKind::Repo)]
#[tokio::test]
async fn modify_team_include_swap_pair(#[case] kind: SubstrateKind) {
    run_with(kind, |workspace| async move {
        workspace.insert(a_minimal_role("eng-lead")).await.unwrap();
        workspace.insert(a_minimal_role("designer")).await.unwrap();
        workspace.insert(a_minimal_team("platform")).await.unwrap();
        workspace.insert(a_minimal_team("ops")).await.unwrap();
        workspace
            .insert(a_team_with_composition(
                "eng",
                &[("platform", "eng-lead")],
                &[],
            ))
            .await
            .unwrap();
        workspace.persist().await.unwrap();

        let mut team = workspace.checkout(team_ref("eng")).await.unwrap();
        team.set_include(Some(vec![(
            EntityRef::new("ops"),
            EntityRef::new("designer"),
        )]))
        .await
        .unwrap();
        team.commit().await.unwrap();
        workspace.persist().await.unwrap();

        let team = workspace.resolve(team_ref("eng")).await.unwrap();
        let include = team.include().await.unwrap().expect("include populated");
        assert_eq!(include.len(), 1);
        assert_eq!(include[0].0.id(), "ops");
        assert_eq!(include[0].1.id(), "designer");
    })
    .await;
}
