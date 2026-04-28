//! User job: modify a persisted entity.
//!
//! Take an entity that is already in the substrate (or freshly inserted
//! and persisted), check it out, change a field, commit, persist, and
//! observe the new value via a fresh resolve. For repo substrates the
//! on-disk artifact is part of the public output and is asserted
//! against alongside the API result.

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
        relay::a_minimal_relay,
        reusable_workflow::a_reusable_workflow_with_review_step,
        role::a_minimal_role,
        team::{a_minimal_team, a_team_with_composition},
        workflow::a_workflow_with_review_placeholder,
    },
};

fn role_ref(id: &str) -> AnyEntityRef {
    AnyEntityRef::Role(EntityRef::new(id))
}

fn relay_ref(id: &str, workflow_id: &str) -> AnyEntityRef {
    let parent = WorkflowParent::Workflow(EntityRef::<Workflow>::new(workflow_id));
    AnyEntityRef::Relay(EntityRef::with_parent(id, parent))
}

fn team_ref(id: &str) -> AnyEntityRef {
    AnyEntityRef::Team(EntityRef::new(id))
}

#[rstest]
#[case::in_memory(SubstrateKind::InMemory)]
#[case::repo(SubstrateKind::Repo)]
#[tokio::test]
async fn modify_required_field_within_session(#[case] kind: SubstrateKind) {
    run_with(kind, || async {
        EntityClient::insert(a_minimal_role("eng-lead"))
            .await
            .unwrap();
        EntityClient::persist().await.unwrap();

        let mut entity = EntityClient::checkout(role_ref("eng-lead")).await.unwrap();
        if let TrackedEntity::Role(ref mut r) = entity {
            r.set_name("Engineering Lead".to_string()).await.unwrap();
        }
        entity.commit().await.unwrap();
        EntityClient::persist().await.unwrap();

        let resolved = EntityClient::resolve(role_ref("eng-lead")).await.unwrap();
        let TrackedEntity::Role(role) = resolved else {
            panic!("expected Role")
        };
        assert_eq!(role.name().await.unwrap(), "Engineering Lead");
    })
    .await;
}

#[rstest]
#[case::in_memory(SubstrateKind::InMemory)]
#[case::repo(SubstrateKind::Repo)]
#[tokio::test]
async fn modify_optional_field_within_session(#[case] kind: SubstrateKind) {
    run_with(kind, || async {
        EntityClient::insert(a_minimal_role("eng-lead"))
            .await
            .unwrap();
        EntityClient::persist().await.unwrap();

        // None -> Some
        let mut entity = EntityClient::checkout(role_ref("eng-lead")).await.unwrap();
        if let TrackedEntity::Role(ref mut r) = entity {
            r.set_description(Some("Owns delivery.".to_string()))
                .await
                .unwrap();
        }
        entity.commit().await.unwrap();
        EntityClient::persist().await.unwrap();

        let resolved = EntityClient::resolve(role_ref("eng-lead")).await.unwrap();
        let TrackedEntity::Role(role) = resolved else {
            panic!("expected Role")
        };
        assert_eq!(role.description().await.unwrap(), Some("Owns delivery."));

        // Some -> None
        let mut entity = EntityClient::checkout(role_ref("eng-lead")).await.unwrap();
        if let TrackedEntity::Role(ref mut r) = entity {
            r.set_description(None).await.unwrap();
        }
        entity.commit().await.unwrap();
        EntityClient::persist().await.unwrap();

        let resolved = EntityClient::resolve(role_ref("eng-lead")).await.unwrap();
        let TrackedEntity::Role(role) = resolved else {
            panic!("expected Role")
        };
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
    let role_file = path.join("roles/eng-lead.md");

    pari::with(RepoSubstrate::new(path.clone()).unwrap(), || async {
        EntityClient::insert(a_minimal_role("eng-lead"))
            .await
            .unwrap();
        EntityClient::persist().await.unwrap();
    })
    .await;

    pari::with(RepoSubstrate::new(path.clone()).unwrap(), || async {
        EntityClient::resolve(role_ref("eng-lead")).await.unwrap();
        let mut entity = EntityClient::checkout(role_ref("eng-lead")).await.unwrap();
        if let TrackedEntity::Role(ref mut r) = entity {
            r.set_name("Engineering Lead".to_string()).await.unwrap();
        }
        entity.commit().await.unwrap();
        EntityClient::persist().await.unwrap();
    })
    .await;

    // Third session: confirm the new value is observable through resolve.
    pari::with(RepoSubstrate::new(path.clone()).unwrap(), || async {
        let resolved = EntityClient::resolve(role_ref("eng-lead")).await.unwrap();
        let TrackedEntity::Role(role) = resolved else {
            panic!("expected Role")
        };
        assert_eq!(role.name().await.unwrap(), "Engineering Lead");
    })
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
    run_with(kind, || async {
        // Roles for raci + review approver on each reusable workflow.
        EntityClient::insert(a_minimal_role("eng-lead"))
            .await
            .unwrap();
        EntityClient::insert(a_minimal_role("approver"))
            .await
            .unwrap();

        // Two reusable workflows the relay can delegate to. Both share
        // state ids so the relay's state_map stays valid across the swap.
        EntityClient::insert(a_reusable_workflow_with_review_step(
            "ApprovalAlpha",
            "eng-lead",
            "approver",
        ))
        .await
        .unwrap();
        EntityClient::insert(a_reusable_workflow_with_review_step(
            "ApprovalBeta",
            "eng-lead",
            "approver",
        ))
        .await
        .unwrap();

        // Parent workflow shell so the relay's parent exists when its
        // entity_ref is cross-entity-validated.
        EntityClient::insert(a_workflow_with_review_placeholder(
            "DesignFlow",
            "eng-lead",
            "approver",
        ))
        .await
        .unwrap();
        EntityClient::insert(a_minimal_relay(
            "Handoff",
            "DesignFlow",
            "eng-lead",
            "ApprovalAlpha",
        ))
        .await
        .unwrap();
        EntityClient::persist().await.unwrap();

        let mut entity = EntityClient::checkout(relay_ref("Handoff", "DesignFlow"))
            .await
            .unwrap();
        if let TrackedEntity::Relay(ref mut r) = entity {
            r.set_delegates_to(EntityRef::new("ApprovalBeta"))
                .await
                .unwrap();
        }
        entity.commit().await.unwrap();
        EntityClient::persist().await.unwrap();

        let resolved = EntityClient::resolve(relay_ref("Handoff", "DesignFlow"))
            .await
            .unwrap();
        let TrackedEntity::Relay(relay) = resolved else {
            panic!("expected Relay")
        };
        assert_eq!(relay.delegates_to().await.unwrap().id(), "ApprovalBeta");
    })
    .await;
}

#[rstest]
#[case::in_memory(SubstrateKind::InMemory)]
#[case::repo(SubstrateKind::Repo)]
#[tokio::test]
async fn modify_team_include_swap_pair(#[case] kind: SubstrateKind) {
    run_with(kind, || async {
        // Two role candidates and three teams: the team being modified
        // plus the two teams it can include.
        EntityClient::insert(a_minimal_role("eng-lead"))
            .await
            .unwrap();
        EntityClient::insert(a_minimal_role("designer"))
            .await
            .unwrap();
        EntityClient::insert(a_minimal_team("platform"))
            .await
            .unwrap();
        EntityClient::insert(a_minimal_team("ops")).await.unwrap();
        EntityClient::insert(a_team_with_composition(
            "eng",
            &[("platform", "eng-lead")],
            &[],
        ))
        .await
        .unwrap();
        EntityClient::persist().await.unwrap();

        let mut entity = EntityClient::checkout(team_ref("eng")).await.unwrap();
        if let TrackedEntity::Team(ref mut t) = entity {
            t.set_include(Some(vec![(
                EntityRef::new("ops"),
                EntityRef::new("designer"),
            )]))
            .await
            .unwrap();
        }
        entity.commit().await.unwrap();
        EntityClient::persist().await.unwrap();

        let resolved = EntityClient::resolve(team_ref("eng")).await.unwrap();
        let TrackedEntity::Team(team) = resolved else {
            panic!("expected Team")
        };
        let include = team.include().await.unwrap().expect("include populated");
        assert_eq!(include.len(), 1);
        assert_eq!(include[0].0.id(), "ops");
        assert_eq!(include[0].1.id(), "designer");
    })
    .await;
}
