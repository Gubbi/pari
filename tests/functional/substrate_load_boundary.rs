//! Substrate-side behavior at the load boundary.
//!
//! Two related concerns covered in one file:
//!
//! - **Schema gate rejection.** A persistence artifact may parse cleanly
//!   through the codec yet carry a JSON slice that violates the
//!   projected entity schema (missing required field, wrong type,
//!   unknown top-level key). The substrate gate rejects such slices
//!   before they reach the tracked-entity merge so the workspace surface
//!   never sees a malformed entity.
//! - **Extensions `x-` prefix at the disk boundary.** The on-disk
//!   artifact carries `x-`-prefixed keys; the loaded entity exposes bare
//!   keys. Round-trip: insert with bare keys → on-disk artifact carries
//!   `x-` prefix → load translates back to bare keys.
//!
//! `RepoSubstrate`-only — both concerns are shaped against on-disk
//! artifacts. The harness mirrors `external_corruption.rs`: produce a
//! healthy file via insert + persist, mutate or inspect on disk, then
//! `forget` and re-resolve to force a refetch.

use pari::{
    entities::role::Role,
    entity::EntityRef,
    error::{primitive::PrimitiveError, ActivityError},
    substrate::RepoSubstrate,
    workspace::Workspace,
};
use serde_json::json;
use tempfile::TempDir;

use crate::{common::substrate::with_workspace, fixtures::role::a_minimal_role};

fn role_ref(id: &str) -> EntityRef<Role> {
    EntityRef::new(id)
}

async fn trigger_load(workspace: &Workspace, id: &str) -> Result<String, ActivityError> {
    let role = workspace.resolve(role_ref(id)).await?;
    role.name().await.map(|s| s.to_string())
}

/// A bad on-disk artifact must surface as `UnpersistableDefinition`
/// regardless of which substrate layer caught it — the codec rejects
/// some shapes before the slice ever reaches the schema gate (e.g. a
/// frontmatter key no flatten slot accepts), and the schema gate
/// rejects the rest. The assertion matches either shape and pins the
/// offending field/value in the rendered error.
fn assert_load_rejection<T: std::fmt::Debug>(result: Result<T, ActivityError>, needle: &str) {
    let err = result.err().expect("expected an error");
    let cause = match &err {
        ActivityError::UnpersistableDefinition { cause, .. } => cause,
        _ => panic!("expected UnpersistableDefinition, got: {err:?}"),
    };
    let rendered = format!("{cause:?}");
    assert!(
        matches!(
            cause,
            PrimitiveError::PartialPayloadDeserialization { .. }
                | PrimitiveError::UnsupportedSlotComposition { .. }
        ),
        "expected codec or schema-gate rejection, got: {rendered}"
    );
    assert!(
        rendered.to_lowercase().contains(&needle.to_lowercase()),
        "expected rejection to mention '{needle}', got: {rendered}"
    );
}

/// Replace the YAML frontmatter block in the persisted role file with a
/// fresh map. Body (H1, description) is preserved.
fn rewrite_frontmatter(file: &std::path::Path, frontmatter: &serde_yaml::Mapping) {
    let raw = std::fs::read_to_string(file).unwrap();
    let body_start = raw[4..]
        .find("\n---\n")
        .map(|i| 4 + i + "\n---\n".len())
        .expect("expected closing frontmatter fence");
    let body = &raw[body_start..];
    let yaml = serde_yaml::to_string(frontmatter).unwrap();
    let next = format!("---\n{yaml}---\n{body}");
    std::fs::write(file, next).unwrap();
}

// ===========================================================================
// Schema-gate rejection (#6)
// ===========================================================================

/// An absent required frontmatter key surfaces as a schema-gate
/// rejection. The codec emits `null` for the missing key (it doesn't
/// know whether the key is required); the projected schema's
/// `type: string` constraint catches it.
#[tokio::test]
async fn load_rejects_artifact_with_missing_required_field() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().to_path_buf();
    let role_file = path.join("common/roles/eng-lead.md");

    with_workspace(
        RepoSubstrate::new(path.clone()).unwrap(),
        |workspace| async move {
            workspace.insert(a_minimal_role("eng-lead")).await.unwrap();
            workspace.persist().await.unwrap();

            // Empty frontmatter — purpose is the only key the minimal
            // role writes; stripping it yields a null `purpose` slice
            // entry.
            let fm = serde_yaml::Mapping::new();
            rewrite_frontmatter(&role_file, &fm);

            workspace.forget(role_ref("eng-lead")).await.unwrap();

            assert_load_rejection(trigger_load(&workspace, "eng-lead").await, "string");
        },
    )
    .await;
}

#[tokio::test]
async fn load_rejects_artifact_with_wrong_field_type() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().to_path_buf();
    let role_file = path.join("common/roles/eng-lead.md");

    with_workspace(
        RepoSubstrate::new(path.clone()).unwrap(),
        |workspace| async move {
            // Persist a role whose `traits` is a populated array on disk.
            let mut role = a_minimal_role("eng-lead");
            role.traits = Some(vec!["accountable".to_string()]);
            workspace.insert(role).await.unwrap();
            workspace.persist().await.unwrap();

            // Replace `traits` with a scalar string. Codec yields
            // `traits: "broken"`; schema requires array.
            let mut fm = serde_yaml::Mapping::new();
            fm.insert(
                serde_yaml::Value::String("purpose".to_string()),
                serde_yaml::Value::String("test purpose".to_string()),
            );
            fm.insert(
                serde_yaml::Value::String("traits".to_string()),
                serde_yaml::Value::String("broken".to_string()),
            );
            rewrite_frontmatter(&role_file, &fm);

            workspace.forget(role_ref("eng-lead")).await.unwrap();

            assert_load_rejection(trigger_load(&workspace, "eng-lead").await, "array");
        },
    )
    .await;
}

#[tokio::test]
async fn load_rejects_artifact_with_unknown_field() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().to_path_buf();
    let role_file = path.join("common/roles/eng-lead.md");

    with_workspace(
        RepoSubstrate::new(path.clone()).unwrap(),
        |workspace| async move {
            workspace.insert(a_minimal_role("eng-lead")).await.unwrap();
            workspace.persist().await.unwrap();

            // Add a bare (non-`x-`) frontmatter key. The codec routes it
            // into the flattened extensions slot, the merge step lifts it
            // back to the top level of the slice, and the projected
            // schema rejects it via additionalProperties:false (no
            // patternProperties match).
            let mut fm = serde_yaml::Mapping::new();
            fm.insert(
                serde_yaml::Value::String("purpose".to_string()),
                serde_yaml::Value::String("test purpose".to_string()),
            );
            fm.insert(
                serde_yaml::Value::String("rogue".to_string()),
                serde_yaml::Value::String("value".to_string()),
            );
            rewrite_frontmatter(&role_file, &fm);

            workspace.forget(role_ref("eng-lead")).await.unwrap();

            assert_load_rejection(trigger_load(&workspace, "eng-lead").await, "rogue");
        },
    )
    .await;
}

// ===========================================================================
// Extensions `x-` prefix at the disk boundary (#7)
// ===========================================================================

// TODO: re-enable after the codec/slot refactor lands. Tracked::Serialize
// flattens extensions to top-level x- keys post-d0f41fe; the codec still
// looks up a nested "extensions" field and writes nothing.

#[tokio::test]
async fn repo_substrate_writes_x_prefixed_extension_keys_to_disk() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().to_path_buf();
    let role_file = path.join("common/roles/eng-lead.md");

    with_workspace(
        RepoSubstrate::new(path.clone()).unwrap(),
        |workspace| async move {
            // In-memory bag holds bare keys.
            let mut role = a_minimal_role("eng-lead");
            role.extensions.0.insert("color".to_string(), json!("red"));
            role.extensions.0.insert("priority".to_string(), json!(7));

            workspace.insert(role).await.unwrap();
            workspace.persist().await.unwrap();

            let raw = std::fs::read_to_string(&role_file).unwrap();
            assert!(
                raw.contains("x-color: red"),
                "expected x-color in frontmatter, got:\n{raw}"
            );
            assert!(
                raw.contains("x-priority: 7"),
                "expected x-priority in frontmatter, got:\n{raw}"
            );
            assert!(
                !raw.contains("\ncolor: red") && !raw.contains("\npriority: 7"),
                "frontmatter should not carry bare extension keys, got:\n{raw}"
            );
        },
    )
    .await;
}

// TODO: re-enable after the codec/slot refactor lands. Today
// viewer.extensions() triggers Load{field:"extensions"}, which the
// load-side field-selection gate rejects as not-in-validation-schema.

#[tokio::test]
async fn repo_substrate_loads_x_prefixed_disk_keys_as_bare_keys() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().to_path_buf();
    let role_file = path.join("common/roles/eng-lead.md");

    with_workspace(
        RepoSubstrate::new(path.clone()).unwrap(),
        |workspace| async move {
            workspace.insert(a_minimal_role("eng-lead")).await.unwrap();
            workspace.persist().await.unwrap();

            // Hand-edit the on-disk frontmatter to introduce x-prefixed
            // extensions. Bare-key behavior is enforced by the gate; the
            // loader's job is to translate the prefix back.
            let mut fm = serde_yaml::Mapping::new();
            fm.insert(
                serde_yaml::Value::String("purpose".to_string()),
                serde_yaml::Value::String("test purpose".to_string()),
            );
            fm.insert(
                serde_yaml::Value::String("x-color".to_string()),
                serde_yaml::Value::String("red".to_string()),
            );
            fm.insert(
                serde_yaml::Value::String("x-priority".to_string()),
                serde_yaml::Value::Number(7.into()),
            );
            rewrite_frontmatter(&role_file, &fm);

            workspace.forget(role_ref("eng-lead")).await.unwrap();

            let role = workspace.resolve(role_ref("eng-lead")).await.unwrap();
            let extensions = role.extensions().await.unwrap();
            assert_eq!(extensions.0.get("color"), Some(&json!("red")));
            assert_eq!(extensions.0.get("priority"), Some(&json!(7)));
            assert!(
                !extensions.0.contains_key("x-color") && !extensions.0.contains_key("x-priority"),
                "in-memory bag must not retain x- prefix, got: {:?}",
                extensions.0
            );
        },
    )
    .await;
}
