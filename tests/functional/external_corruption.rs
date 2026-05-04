//! User job: corrupted substrate artifacts surface a structured error.
//!
//! When something outside Pari has produced an unparseable artifact —
//! a hand-edited markdown file with broken YAML frontmatter, a missing
//! required slot, a never-closed frontmatter fence — the next load
//! must fail with `ActivityError::UnpersistableDefinition` carrying
//! the codec primitive that classified the failure, not panic and not
//! return success with garbage. `RepoSubstrate`-only because the
//! corruption is shaped against on-disk artifacts.
//!
//! Each scenario produces a healthy file via `insert` + `persist`,
//! mutates the on-disk artifact, then `unload`s and re-resolves to
//! force a refetch through the codec.

use pari::{
    entity::{AnyEntityRef, EntityRef, TrackedEntity},
    error::{primitive::PrimitiveError, ActivityError},
    substrate::RepoSubstrate,
    workspace::EntityClient,
};
use tempfile::TempDir;

use crate::fixtures::role::a_minimal_role;

fn role_ref(id: &str) -> AnyEntityRef {
    AnyEntityRef::Role(EntityRef::new(id))
}

/// Resolve and trigger the codec by reading a field. Field accessors
/// drive the lazy load — `resolve` alone does not.
async fn trigger_load(id: &str) -> Result<String, ActivityError> {
    let resolved = EntityClient::resolve(role_ref(id)).await?;
    let TrackedEntity::Role(role) = resolved else {
        panic!("expected Role")
    };
    role.name().await.map(|s| s.to_string())
}

fn assert_unpersistable<T>(
    result: Result<T, ActivityError>,
    matches: impl Fn(&PrimitiveError) -> bool,
) {
    let err = result.err().expect("expected an error");
    let cause = match &err {
        ActivityError::UnpersistableDefinition { cause, .. } => cause,
        _ => panic!("expected UnpersistableDefinition, got: {err:?}"),
    };
    assert!(matches(cause), "unexpected primitive cause: {cause:?}");
}

/// After persisting a healthy role, scribble invalid YAML inside the
/// frontmatter fence. The next refetch surfaces
/// `MalformedFrontmatter`.
#[tokio::test]
async fn malformed_yaml_frontmatter_surfaces_codec_error() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().to_path_buf();
    let role_file = path.join("common/roles/eng-lead.md");

    pari::with(RepoSubstrate::new(path.clone()).unwrap(), || async {
        EntityClient::insert(a_minimal_role("eng-lead"))
            .await
            .unwrap();
        EntityClient::persist().await.unwrap();

        let original = std::fs::read_to_string(&role_file).unwrap();
        let corrupted = original.replacen("---\n", "---\nname: : not: valid: yaml\n", 1);
        std::fs::write(&role_file, corrupted).unwrap();

        EntityClient::forget(role_ref("eng-lead")).await.unwrap();

        let result = trigger_load("eng-lead").await;
        assert_unpersistable(result, |e| {
            matches!(e, PrimitiveError::MalformedFrontmatter { .. })
        });
    })
    .await;
}

/// Strip the closing frontmatter fence so the parser cannot terminate
/// the block. The next refetch surfaces `MalformedFrontmatter`.
#[tokio::test]
async fn unterminated_frontmatter_surfaces_codec_error() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().to_path_buf();
    let role_file = path.join("common/roles/eng-lead.md");

    pari::with(RepoSubstrate::new(path.clone()).unwrap(), || async {
        EntityClient::insert(a_minimal_role("eng-lead"))
            .await
            .unwrap();
        EntityClient::persist().await.unwrap();

        let original = std::fs::read_to_string(&role_file).unwrap();
        // Drop the second `---` fence entirely.
        let (head, tail) = original
            .split_once("---\n")
            .and_then(|(h, rest)| {
                rest.split_once("\n---\n")
                    .map(|(fm, body)| (format!("{h}---\n{fm}\n"), body.to_string()))
            })
            .expect("expected frontmatter fences in persisted role");
        std::fs::write(&role_file, format!("{head}{tail}")).unwrap();

        EntityClient::forget(role_ref("eng-lead")).await.unwrap();

        let result = trigger_load("eng-lead").await;
        assert_unpersistable(result, |e| {
            matches!(e, PrimitiveError::MalformedFrontmatter { .. })
        });
    })
    .await;
}

/// Replace the file body with content that has no `---` fence at all
/// and no H1 — the codec cannot extract the slots it needs.
#[tokio::test]
async fn missing_required_shape_surfaces_codec_error() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().to_path_buf();
    let role_file = path.join("common/roles/eng-lead.md");

    pari::with(RepoSubstrate::new(path.clone()).unwrap(), || async {
        EntityClient::insert(a_minimal_role("eng-lead"))
            .await
            .unwrap();
        EntityClient::persist().await.unwrap();

        std::fs::write(&role_file, "this is not a role file\n").unwrap();

        EntityClient::forget(role_ref("eng-lead")).await.unwrap();

        let result = trigger_load("eng-lead").await;
        // Don't pin the exact primitive variant — the codec is free to
        // classify "no fence + no H1" as either malformed frontmatter
        // or a missing-slot error. Pin only that the activity tier is
        // UnpersistableDefinition and the cause is a codec primitive.
        assert_unpersistable(result, |_| true);
    })
    .await;
}
