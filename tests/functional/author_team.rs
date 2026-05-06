//! User job: author a team.
//!
//! A new team is defined and persisted; cross-entity refs (members'
//! roles, included teams, imported teams) must already exist when the
//! team is inserted because cross-entity validation runs at insert.

use pari::{entities::team::Team, entity::EntityRef, substrate::RepoSubstrate};
use rstest::rstest;
use tempfile::TempDir;

use crate::{
    common::substrate::{run_with, with_workspace, SubstrateKind},
    fixtures::{
        role::a_minimal_role,
        team::{a_minimal_team, a_team_with_composition, a_team_with_members},
    },
};

fn team_ref(id: &str) -> EntityRef<Team> {
    EntityRef::new(id)
}

#[rstest]
#[case::in_memory(SubstrateKind::InMemory)]
#[case::repo(SubstrateKind::Repo)]
#[tokio::test]
async fn minimal_team_is_observable_after_persist(#[case] kind: SubstrateKind) {
    run_with(kind, |workspace| async move {
        workspace.insert(a_minimal_team("eng")).await.unwrap();
        workspace.persist().await.unwrap();

        let team = workspace.resolve(team_ref("eng")).await.unwrap();
        assert_eq!(team.name().await.unwrap(), "Minimal Team");
        assert_eq!(team.description().await.unwrap(), Some("A team for tests."));
        assert!(team.members().await.unwrap().is_none());
        assert!(team.include().await.unwrap().is_none());
        assert!(team.import().await.unwrap().is_none());
    })
    .await;
}

#[rstest]
#[case::in_memory(SubstrateKind::InMemory)]
#[case::repo(SubstrateKind::Repo)]
#[tokio::test]
async fn team_with_members_is_observable_after_persist(#[case] kind: SubstrateKind) {
    run_with(kind, |workspace| async move {
        // Roles must exist before the team's cross-entity validation runs.
        workspace.insert(a_minimal_role("eng-lead")).await.unwrap();
        workspace.insert(a_minimal_role("designer")).await.unwrap();

        workspace
            .insert(a_team_with_members(
                "eng",
                &[("@alice", "eng-lead"), ("@bob", "designer")],
            ))
            .await
            .unwrap();
        workspace.persist().await.unwrap();

        let team = workspace.resolve(team_ref("eng")).await.unwrap();
        let members = team
            .members()
            .await
            .unwrap()
            .expect("members populated")
            .to_vec();
        assert_eq!(members.len(), 2);
        assert_eq!(members[0].handle, "@alice");
        assert_eq!(members[0].role.id(), "eng-lead");
        assert_eq!(members[1].handle, "@bob");
        assert_eq!(members[1].role.id(), "designer");
    })
    .await;
}

/// Full team — members + include + import — round-trips a fresh
/// [`RepoSubstrate`] over the same on-disk directory.
#[tokio::test]
async fn team_round_trips_repo_substrate_across_sessions() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().to_path_buf();

    with_workspace(
        RepoSubstrate::new(path.clone()).unwrap(),
        |workspace| async move {
            // Prerequisites: roles for members; teams for include/import.
            workspace.insert(a_minimal_role("eng-lead")).await.unwrap();
            workspace.insert(a_minimal_role("designer")).await.unwrap();
            workspace.insert(a_minimal_team("platform")).await.unwrap();
            workspace.insert(a_minimal_team("ops")).await.unwrap();

            workspace
                .insert(a_team_with_members(
                    "core",
                    &[("@alice", "eng-lead"), ("@bob", "designer")],
                ))
                .await
                .unwrap();
            workspace.persist().await.unwrap();

            // Authored separately because composition references "core".
            workspace
                .insert(a_team_with_composition(
                    "eng",
                    &[("platform", "eng-lead")],
                    &["ops"],
                ))
                .await
                .unwrap();
            workspace.persist().await.unwrap();
        },
    )
    .await;

    with_workspace(
        RepoSubstrate::new(path.clone()).unwrap(),
        |workspace| async move {
            let team = workspace.resolve(team_ref("eng")).await.unwrap();
            assert_eq!(team.name().await.unwrap(), "Composed Team");

            let include = team.include().await.unwrap().expect("include populated");
            assert_eq!(include.len(), 1);
            let (team_key, role_val) = &include[0];
            assert_eq!(team_key.id(), "platform");
            assert_eq!(role_val.id(), "eng-lead");

            let import = team
                .import()
                .await
                .unwrap()
                .expect("import populated")
                .to_vec();
            assert_eq!(import.len(), 1);
            assert_eq!(import[0].id(), "ops");
        },
    )
    .await;
}

/// `RepoSubstrate` writes `common/teams/<id>.md` in the format external tools
/// consume: H1 with the name, description paragraph, and frontmatter
/// keys for members/include/import.
#[tokio::test]
async fn repo_substrate_writes_expected_team_file() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().to_path_buf();
    let team_file = path.join("common/teams/core.md");

    with_workspace(
        RepoSubstrate::new(path.clone()).unwrap(),
        |workspace| async move {
            workspace.insert(a_minimal_role("eng-lead")).await.unwrap();
            workspace.insert(a_minimal_role("designer")).await.unwrap();
            workspace
                .insert(a_team_with_members(
                    "core",
                    &[("@alice", "eng-lead"), ("@bob", "designer")],
                ))
                .await
                .unwrap();
            workspace.persist().await.unwrap();
        },
    )
    .await;

    assert!(team_file.exists(), "expected {team_file:?} to be created");
    let contents = std::fs::read_to_string(&team_file).unwrap();

    assert!(
        contents.contains("# Engineering Team"),
        "expected H1 with team name, got:\n{contents}"
    );
    assert!(
        contents.contains("A team with a roster."),
        "expected description paragraph, got:\n{contents}"
    );
    assert!(
        contents.contains("members:"),
        "expected members frontmatter key, got:\n{contents}"
    );
    assert!(
        contents.contains("@alice"),
        "expected @alice handle, got:\n{contents}"
    );
    assert!(
        contents.contains("eng-lead"),
        "expected eng-lead role id, got:\n{contents}"
    );
}
