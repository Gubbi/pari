use pari::substrate::repo::RepoSubstrate;
use pari::substrate::repo::codec::RepoCodec;
use pari::substrate::pipeline::{Codec, FieldMapping};
use pari::substrate::repo::slot::{RepoSlot, SectionContent};
use std::collections::HashMap;
use tempfile::TempDir;

// ---------------------------------------------------------------------------
// RepoCodec — decode
// ---------------------------------------------------------------------------

#[test]
fn codec_decode_h1_field() {
    let codec = RepoCodec;
    let content = "---\npurpose: test purpose\n---\n\n# Engineering Lead\n\nSome description.\n";
    let schema = &[
        FieldMapping { key: "name",    slot: RepoSlot::H1 },
        FieldMapping { key: "purpose", slot: RepoSlot::FrontmatterKey("purpose") },
    ];
    let fields = codec.decode(&content.to_string(), schema).unwrap();
    assert_eq!(fields.get("name").and_then(|v| v.as_str()), Some("Engineering Lead"));
    assert_eq!(fields.get("purpose").and_then(|v| v.as_str()), Some("test purpose"));
}

#[test]
fn codec_decode_description_paragraph() {
    let codec = RepoCodec;
    let content = "---\n---\n\n# Role\n\nThis is the description.\n\n## Section\n\nContent.\n";
    let schema = &[
        FieldMapping { key: "name",        slot: RepoSlot::H1 },
        FieldMapping { key: "description", slot: RepoSlot::DescriptionParagraph },
    ];
    let fields = codec.decode(&content.to_string(), schema).unwrap();
    assert_eq!(fields.get("description").and_then(|v| v.as_str()), Some("This is the description."));
}

#[test]
fn codec_decode_frontmatter_flattened_collects_x_keys() {
    let codec = RepoCodec;
    let content = "---\npurpose: test\nx-owner: alice\nx-priority: high\n---\n\n# Role\n";
    let schema = &[
        FieldMapping { key: "purpose",    slot: RepoSlot::FrontmatterKey("purpose") },
        FieldMapping { key: "extensions", slot: RepoSlot::FrontmatterFlattened },
    ];
    let fields = codec.decode(&content.to_string(), schema).unwrap();
    let extensions = fields.get("extensions").unwrap();
    assert!(extensions.get("x-owner").is_some());
    assert!(extensions.get("x-priority").is_some());
    assert!(extensions.get("purpose").is_none(), "named key must not appear in flattened");
}

#[test]
fn codec_encode_decode_roundtrip() {
    let codec = RepoCodec;
    let schema = &[
        FieldMapping { key: "name",       slot: RepoSlot::H1 },
        FieldMapping { key: "purpose",    slot: RepoSlot::FrontmatterKey("purpose") },
        FieldMapping { key: "extensions", slot: RepoSlot::FrontmatterFlattened },
    ];
    let mut fields = HashMap::new();
    fields.insert("name",       serde_json::json!("Engineering Lead"));
    fields.insert("purpose",    serde_json::json!("Leads engineering"));
    fields.insert("extensions", serde_json::json!({ "x-owner": "alice" }));

    let encoded = codec.encode(&fields, schema).unwrap();
    let decoded = codec.decode(&encoded, schema).unwrap();

    assert_eq!(decoded.get("name").and_then(|v| v.as_str()), Some("Engineering Lead"));
    assert_eq!(decoded.get("purpose").and_then(|v| v.as_str()), Some("Leads engineering"));
    assert_eq!(
        decoded.get("extensions").and_then(|e| e.get("x-owner")).and_then(|v| v.as_str()),
        Some("alice"),
    );
}

#[test]
fn codec_decode_bullet_list_section() {
    let codec = RepoCodec;
    let content = "---\n---\n\n# Hook\n\n## Instructions\n\n- Step one\n- Step two\n";
    let schema = &[
        FieldMapping { key: "name",         slot: RepoSlot::H1 },
        FieldMapping { key: "instructions", slot: RepoSlot::Section("Instructions", SectionContent::BulletList) },
    ];
    let fields = codec.decode(&content.to_string(), schema).unwrap();
    let instructions = fields.get("instructions").unwrap();
    assert_eq!(instructions, &serde_json::json!(["Step one", "Step two"]));
}

// ---------------------------------------------------------------------------
// RepoLocationResolver
// ---------------------------------------------------------------------------

use pari::substrate::repo::resolver::RepoLocationResolver;
use pari::substrate::pipeline::LocationResolver;

#[test]
fn resolver_expands_id_template() {
    let root = std::path::PathBuf::from("/repo");
    let resolver = RepoLocationResolver::new(root.clone());
    let entity = serde_json::json!({ "entity_ref": { "id": "eng-lead", "kind": "Role" } });
    let path = resolver.resolve("roles/{id}.md", &entity);
    assert_eq!(path, root.join("roles/eng-lead.md"));
}

#[test]
fn resolver_expands_parent_base_template() {
    let root = std::path::PathBuf::from("/repo");
    let resolver = RepoLocationResolver::new(root.clone());
    let entity = serde_json::json!({
        "entity_ref": { "id": "WriteProposal", "kind": "Task", "workflow_id": "InitiativeWorkflow" }
    });
    let path = resolver.resolve("{parent.base}/{id}/README.md", &entity);
    assert_eq!(path, root.join("workflows/InitiativeWorkflow/WriteProposal/README.md"));
}

// ---------------------------------------------------------------------------
// compute_lca
// ---------------------------------------------------------------------------

use pari::substrate::repo::executor::compute_lca;

#[test]
fn lca_of_single_path_is_parent_dir() {
    let paths = vec![std::path::Path::new("workflows/InitiativeWorkflow/WriteProposal/README.md")];
    let lca = compute_lca(&paths);
    assert_eq!(lca, std::path::Path::new("workflows/InitiativeWorkflow/WriteProposal"));
}

#[test]
fn lca_of_sibling_files_is_parent_dir() {
    let paths = vec![
        std::path::Path::new("workflows/InitiativeWorkflow/WriteProposal/README.md"),
        std::path::Path::new("workflows/InitiativeWorkflow/HandoffToClient/README.md"),
    ];
    let lca = compute_lca(&paths);
    assert_eq!(lca, std::path::Path::new("workflows/InitiativeWorkflow"));
}

#[test]
fn lca_of_workflow_and_task_files() {
    let paths = vec![
        std::path::Path::new("workflows/InitiativeWorkflow/README.md"),
        std::path::Path::new("workflows/InitiativeWorkflow/WriteProposal/README.md"),
    ];
    let lca = compute_lca(&paths);
    assert_eq!(lca, std::path::Path::new("workflows/InitiativeWorkflow"));
}

// ---------------------------------------------------------------------------
// RepoSubstrate integration
// ---------------------------------------------------------------------------

#[tokio::test]
async fn repo_substrate_write_and_read_role_roundtrip() {
    use pari::store::{EntityServer, EntityClient};
    use pari::entity::{AnyEntityRef, EntityRef, StoreEntity};
    use pari::entities::role::{Role, TrackedRole};

    let dir = TempDir::new().unwrap();
    let substrate = RepoSubstrate::new(dir.path().to_path_buf()).unwrap();

    EntityServer::with_test(substrate, || async {
        let role = Role {
            entity_ref:  EntityRef::new("eng-lead"),
            name:        "Engineering Lead".to_string(),
            description: None,
            purpose:     "Leads engineering".to_string(),
            traits:      Some(vec!["reviewer".to_string()]),
            extensions:  HashMap::new(),
        };

        // Insert and persist
        EntityClient::insert(StoreEntity::from_role(TrackedRole::from(role))).await.unwrap();
        EntityClient::persist().await.unwrap();

        // Verify file exists
        let role_path = dir.path().join("roles/eng-lead.md");
        assert!(role_path.exists(), "Role file should be created on persist");

        // Read back — unload and re-resolve
        EntityClient::unload(AnyEntityRef::Role(EntityRef::new("eng-lead"))).await.unwrap();
        let resolved = EntityClient::resolve(AnyEntityRef::Role(EntityRef::new("eng-lead"))).await.unwrap();

        if let StoreEntity::Role(r) = resolved {
            let name = r.name().await.unwrap();
            assert_eq!(name, "Engineering Lead");
        } else {
            panic!("expected Role");
        }
    }).await;
}

#[test]
fn repo_substrate_cleanup_stale_dirs_on_startup() {
    let dir = TempDir::new().unwrap();
    // Create stale .part and .old directories
    std::fs::create_dir(dir.path().join("workflows.part")).unwrap();
    std::fs::create_dir(dir.path().join("roles.old")).unwrap();

    let _substrate = RepoSubstrate::new(dir.path().to_path_buf()).unwrap();

    assert!(!dir.path().join("workflows.part").exists(), "stale .part must be cleaned");
    assert!(!dir.path().join("roles.old").exists(), "stale .old must be cleaned");
}
