# Task 11 — RepoSubstrate Implementation

## Scope

Implement the filesystem-backed substrate for the repository layout:

1. `RepoSlot` enum — encoding targets within markdown+YAML frontmatter files
2. `RepoCodec` — encode/decode between field JSON values and markdown+YAML format
3. `RepoLocationResolver` — expand path templates into `PathBuf` locations
4. `RepoExecutor` — execute batched `AssetRequest<PathBuf, String>` with LCA-based atomic swap
5. `RepoSubstrate` — wire the three components + startup cleanup + `Substrate` trait impl
6. `SubstrateSchema<RepoSubstrate>` per entity type — static `EntitySchema<RepoSlot>` for all 9 entity types

---

## Files

- `src/substrate/repo/slot.rs` — `RepoSlot`, `SectionContent`
- `src/substrate/repo/codec.rs` — `RepoCodec`
- `src/substrate/repo/resolver.rs` — `RepoLocationResolver`
- `src/substrate/repo/executor.rs` — `RepoExecutor`, LCA algorithm
- `src/substrate/repo/schemas.rs` — `SubstrateSchema<RepoSubstrate>` impls for all entity types
- `src/substrate/repo/mod.rs` — `RepoSubstrate`, startup cleanup, `Substrate` impl
- `src/substrate/mod.rs` — `pub mod repo;`

---

## Dependencies

- Task 05: All entity types (for SubstrateSchema impls)
- Task 08: Tracked entity serde (used by codec via `serde_json::from_value`)
- Task 10: `Substrate` trait, `Slot`, `LocationResolver`, `Codec`, `Executor`, `AssetRequest`, `AssetResponse`, `AssetOp`, `EntitySchema`, `SubstrateSchema`, `AssetKind`, `FieldMapping`, `MARKDOWN_FILE`, `RAW_FILE`

---

## `RepoSlot` (`src/substrate/repo/slot.rs`)

```rust
use crate::substrate::pipeline::Slot;

#[derive(Clone, Copy)]
pub enum RepoSlot {
    H1,
    FrontmatterKey(&'static str),
    FrontmatterFlattened,
    DescriptionParagraph,
    Section(&'static str, SectionContent),
    FileContent,
}

#[derive(Clone, Copy)]
pub enum SectionContent {
    Paragraph,
    BulletList,
}

impl Slot for RepoSlot {}
```

---

## `RepoCodec` (`src/substrate/repo/codec.rs`)

```rust
use std::collections::HashMap;
use crate::substrate::pipeline::{Codec, CodecError, FieldMapping};
use super::slot::{RepoSlot, SectionContent};

pub struct RepoCodec;

impl Codec for RepoCodec {
    type Slot = RepoSlot;
    type Encoded = String;

    fn encode(
        &self,
        fields: &HashMap<&str, serde_json::Value>,
        schema: &[FieldMapping<RepoSlot>],
    ) -> Result<String, CodecError> {
        // Build output file:
        // 1. Collect FrontmatterKey and FrontmatterFlattened values → YAML frontmatter block
        // 2. H1 heading
        // 3. DescriptionParagraph (if present)
        // 4. Each Section in schema order
        todo!()
    }

    fn decode(
        &self,
        raw: &String,
        schema: &[FieldMapping<RepoSlot>],
    ) -> Result<HashMap<String, serde_json::Value>, CodecError> {
        // Parse markdown:
        // 1. Split frontmatter (--- ... ---) from body
        // 2. Parse YAML frontmatter → serde_yaml::Value
        // 3. For each FieldMapping in schema, extract the field value per slot
        todo!()
    }
}
```

### Markdown Format

```
---
purpose: "Leads engineering initiatives"
traits:
  - reviewer
x-owner: alice
---

# Engineering Lead

Senior technical leader.

## Section Heading

Content here.
```

- YAML frontmatter between `---` delimiters at top of file
- `H1` = first `# ` heading after frontmatter
- `DescriptionParagraph` = first paragraph of body (between H1 and first `##` section or EOF)
- `Section(heading, _)` = content under `## heading`
- `FrontmatterFlattened` = all YAML keys not claimed by named `FrontmatterKey` slots (collects extension `x-*` keys)
- `FileContent` = entire raw file content (for template files)

---

## `RepoLocationResolver` (`src/substrate/repo/resolver.rs`)

```rust
use std::path::{Path, PathBuf};
use crate::substrate::pipeline::LocationResolver;

pub struct RepoLocationResolver {
    root: PathBuf,
}

impl RepoLocationResolver {
    pub fn new(root: PathBuf) -> Self { Self { root } }

    pub fn base_of(location: &Path) -> String {
        location.parent()
            .unwrap_or(location)
            .to_string_lossy()
            .into_owned()
    }
}

impl LocationResolver for RepoLocationResolver {
    type Location = PathBuf;

    fn resolve(&self, path_template: &str, entity: &serde_json::Value) -> PathBuf {
        let expanded = expand_template(path_template, entity);
        self.root.join(expanded)
    }
}

fn expand_template(template: &str, entity: &serde_json::Value) -> String {
    // Replace {id} with entity["entity_ref"]["id"]
    // Replace {parent.base} with entity["entity_ref"]["workflow_id"] resolved to base path
    // Replace {other.field} with entity[field]
    todo!()
}
```

### Template Variables

| Variable | Resolution |
|---|---|
| `{id}` | `entity["entity_ref"]["id"].as_str()` |
| `{parent.base}` | parent entity's directory: for Task/Relay, looks up the parent Workflow's path and calls `base_of` |
| `{field.subfield}` | Nested JSON path access |

---

## `RepoExecutor` (`src/substrate/repo/executor.rs`)

```rust
use std::path::{Path, PathBuf};
use std::collections::HashMap;
use crate::substrate::pipeline::{Executor, AssetRequest, AssetResponse, AssetOp, ExecutorError};

pub struct RepoExecutor {
    root: PathBuf,
}

impl RepoExecutor {
    pub fn new(root: PathBuf) -> Self { Self { root } }
}

impl Executor for RepoExecutor {
    type Location = PathBuf;
    type Encoded  = String;

    fn execute(
        &self,
        ops: Vec<AssetRequest<PathBuf, String>>,
    ) -> Result<Vec<AssetResponse<String>>, Vec<ExecutorError>> {
        // Separate read ops (Get, Head) from write/delete ops
        // Execute reads directly
        // For writes: compute LCA, stage in <lca>.part/, swap atomically
        todo!()
    }
}
```

### LCA-Based Atomic Swap Algorithm

```
execute(ops):

  1. Split: read_ops (Get, Head) and write_ops (Put, Post, Patch, Delete)
  2. Execute read_ops directly — no staging needed
  3. If write_ops non-empty:
     a. Collect write/delete paths
     b. Compute LCA of all write/delete paths
        → smallest directory containing all changed files
     c. Create <lca>.part/ directory
     d. For each file under <lca>/:
        → If file has a write op: write new content into <lca>.part/
        → If file has a delete op: skip (omit from <lca>.part/)
        → Otherwise: hard-link original file into <lca>.part/
     e. Atomic swap:
        fs::rename(<lca>/, <lca>.old/)   // move original aside
        fs::rename(<lca>.part/, <lca>/)  // replace with staged
     f. fs::remove_dir_all(<lca>.old/)  // cleanup
     g. On any error: cleanup <lca>.part/, collect errors, return Err(errors)
  4. Combine and return all responses in original op order
```

### LCA Computation

```rust
/// Find the lowest common ancestor directory of a set of file paths.
fn compute_lca(paths: &[&Path]) -> PathBuf {
    // Split each path into components; find the longest common prefix of directories
    // (not including the filename itself)
    todo!()
}
```

---

## `RepoSubstrate` (`src/substrate/repo/mod.rs`)

```rust
use std::path::{Path, PathBuf};
use crate::substrate::pipeline::Substrate;
use super::{SubstrateError, LoadStrategy};
use crate::entity::EntityKind;
use super::codec::RepoCodec;
use super::executor::RepoExecutor;
use super::resolver::RepoLocationResolver;
use super::schemas::*;  // SubstrateSchema impls

pub struct RepoSubstrate {
    resolver: RepoLocationResolver,
    codec:    RepoCodec,
    executor: RepoExecutor,
}

impl RepoSubstrate {
    pub fn new(root: PathBuf) -> Result<Self, SubstrateError> {
        Self::cleanup_stale(&root)?;
        Ok(Self {
            resolver: RepoLocationResolver::new(root.clone()),
            codec:    RepoCodec,
            executor: RepoExecutor::new(root),
        })
    }

    fn cleanup_stale(root: &Path) -> Result<(), SubstrateError> {
        // Walk root, find directories ending in ".part" or ".old"
        // Remove them unconditionally via fs::remove_dir_all
        // Collect errors, return first error or Ok(())
        todo!()
    }
}

impl Substrate for RepoSubstrate {
    type Slot     = RepoSlot;
    type Location = PathBuf;
    type Encoded  = String;
    type Resolver = RepoLocationResolver;
    type Codec    = RepoCodec;
    type Executor = RepoExecutor;

    fn resolver(&self) -> &RepoLocationResolver { &self.resolver }
    fn codec(&self)    -> &RepoCodec            { &self.codec }
    fn executor(&self) -> &RepoExecutor         { &self.executor }

    fn load_strategy(kind: EntityKind, field: &str) -> LoadStrategy {
        match kind {
            EntityKind::Role             => <Role             as SubstrateSchema<RepoSubstrate>>::SCHEMA.load_strategy_for(field),
            EntityKind::Hook             => <Hook             as SubstrateSchema<RepoSubstrate>>::SCHEMA.load_strategy_for(field),
            EntityKind::Team             => <Team             as SubstrateSchema<RepoSubstrate>>::SCHEMA.load_strategy_for(field),
            EntityKind::ArtifactKind     => <ArtifactKind     as SubstrateSchema<RepoSubstrate>>::SCHEMA.load_strategy_for(field),
            EntityKind::Workflow         => <Workflow         as SubstrateSchema<RepoSubstrate>>::SCHEMA.load_strategy_for(field),
            EntityKind::ReusableWorkflow => <ReusableWorkflow as SubstrateSchema<RepoSubstrate>>::SCHEMA.load_strategy_for(field),
            EntityKind::EmbeddedWorkflow => <EmbeddedWorkflow as SubstrateSchema<RepoSubstrate>>::SCHEMA.load_strategy_for(field),
            EntityKind::Task             => <Task             as SubstrateSchema<RepoSubstrate>>::SCHEMA.load_strategy_for(field),
            EntityKind::Relay            => <Relay            as SubstrateSchema<RepoSubstrate>>::SCHEMA.load_strategy_for(field),
        }
    }

    // persist, load, exists inherited from Substrate trait defaults
}
```

---

## Entity Schemas (`src/substrate/repo/schemas.rs`)

```rust
use crate::substrate::pipeline::{SubstrateSchema, EntitySchema, RefAssetDef, AssetDef, FieldMapping, MARKDOWN_FILE, RAW_FILE};
use super::slot::{RepoSlot, SectionContent};
use super::RepoSubstrate;
use crate::entities::{role::Role, hook::Hook, team::Team, artifact_kind::ArtifactKind,
    workflow::{Workflow, ReusableWorkflow, EmbeddedWorkflow}, task::Task, relay::Relay};

impl SubstrateSchema<RepoSubstrate> for Role {
    const SCHEMA: EntitySchema<RepoSlot> = EntitySchema {
        ref_asset: RefAssetDef {
            path_template: "roles/{id}.md",
            kind: &MARKDOWN_FILE,
            fields: &[
                FieldMapping { key: "name",        slot: RepoSlot::H1 },
                FieldMapping { key: "description",  slot: RepoSlot::DescriptionParagraph },
                FieldMapping { key: "purpose",      slot: RepoSlot::FrontmatterKey("purpose") },
                FieldMapping { key: "traits",       slot: RepoSlot::FrontmatterKey("traits") },
                FieldMapping { key: "extensions",   slot: RepoSlot::FrontmatterFlattened },
            ],
        },
        assets: &[],
    };
}

impl SubstrateSchema<RepoSubstrate> for Hook {
    const SCHEMA: EntitySchema<RepoSlot> = EntitySchema {
        ref_asset: RefAssetDef {
            path_template: "hooks/{id}.md",
            kind: &MARKDOWN_FILE,
            fields: &[
                FieldMapping { key: "name",         slot: RepoSlot::H1 },
                FieldMapping { key: "description",  slot: RepoSlot::DescriptionParagraph },
                FieldMapping { key: "instructions", slot: RepoSlot::Section("Instructions", SectionContent::BulletList) },
                FieldMapping { key: "inputs",       slot: RepoSlot::FrontmatterKey("inputs") },
                FieldMapping { key: "extensions",   slot: RepoSlot::FrontmatterFlattened },
            ],
        },
        assets: &[],
    };
}

impl SubstrateSchema<RepoSubstrate> for Team {
    const SCHEMA: EntitySchema<RepoSlot> = EntitySchema {
        ref_asset: RefAssetDef {
            path_template: "teams/{id}.md",
            kind: &MARKDOWN_FILE,
            fields: &[
                FieldMapping { key: "name",        slot: RepoSlot::H1 },
                FieldMapping { key: "description", slot: RepoSlot::DescriptionParagraph },
                FieldMapping { key: "members",     slot: RepoSlot::FrontmatterKey("members") },
                FieldMapping { key: "include",     slot: RepoSlot::FrontmatterKey("include") },
                FieldMapping { key: "import",      slot: RepoSlot::FrontmatterKey("import") },
                FieldMapping { key: "extensions",  slot: RepoSlot::FrontmatterFlattened },
            ],
        },
        assets: &[],
    };
}

impl SubstrateSchema<RepoSubstrate> for ArtifactKind {
    const SCHEMA: EntitySchema<RepoSlot> = EntitySchema {
        ref_asset: RefAssetDef {
            path_template: "artifact-kinds/{id}.md",
            kind: &MARKDOWN_FILE,
            fields: &[
                FieldMapping { key: "name",        slot: RepoSlot::H1 },
                FieldMapping { key: "description", slot: RepoSlot::DescriptionParagraph },
                FieldMapping { key: "service",     slot: RepoSlot::FrontmatterKey("service") },
                FieldMapping { key: "access",      slot: RepoSlot::FrontmatterKey("access") },
                FieldMapping { key: "guidance",    slot: RepoSlot::Section("Guidance", SectionContent::Paragraph) },
                FieldMapping { key: "extensions",  slot: RepoSlot::FrontmatterFlattened },
            ],
        },
        assets: &[],
    };
}

impl SubstrateSchema<RepoSubstrate> for Workflow {
    const SCHEMA: EntitySchema<RepoSlot> = EntitySchema {
        ref_asset: RefAssetDef {
            path_template: "workflows/{id}/README.md",
            kind: &MARKDOWN_FILE,
            fields: &[
                FieldMapping { key: "name",        slot: RepoSlot::H1 },
                FieldMapping { key: "description", slot: RepoSlot::DescriptionParagraph },
                FieldMapping { key: "purpose",     slot: RepoSlot::Section("Purpose", SectionContent::Paragraph) },
                FieldMapping { key: "raci",        slot: RepoSlot::FrontmatterKey("raci") },
                FieldMapping { key: "steps",       slot: RepoSlot::FrontmatterKey("steps") },
                FieldMapping { key: "states",      slot: RepoSlot::FrontmatterKey("states") },
                FieldMapping { key: "intercepts",  slot: RepoSlot::FrontmatterKey("intercepts") },
                FieldMapping { key: "guidance",    slot: RepoSlot::Section("Guidance", SectionContent::Paragraph) },
                FieldMapping { key: "extensions",  slot: RepoSlot::FrontmatterFlattened },
            ],
        },
        assets: &[],
    };
}

impl SubstrateSchema<RepoSubstrate> for ReusableWorkflow {
    const SCHEMA: EntitySchema<RepoSlot> = EntitySchema {
        ref_asset: RefAssetDef {
            path_template: "reusable-workflows/{id}/README.md",
            kind: &MARKDOWN_FILE,
            fields: &[
                FieldMapping { key: "name",        slot: RepoSlot::H1 },
                FieldMapping { key: "description", slot: RepoSlot::DescriptionParagraph },
                FieldMapping { key: "purpose",     slot: RepoSlot::Section("Purpose", SectionContent::Paragraph) },
                FieldMapping { key: "raci",        slot: RepoSlot::FrontmatterKey("raci") },
                FieldMapping { key: "steps",       slot: RepoSlot::FrontmatterKey("steps") },
                FieldMapping { key: "states",      slot: RepoSlot::FrontmatterKey("states") },
                FieldMapping { key: "intercepts",  slot: RepoSlot::FrontmatterKey("intercepts") },
                FieldMapping { key: "guidance",    slot: RepoSlot::Section("Guidance", SectionContent::Paragraph) },
                FieldMapping { key: "extensions",  slot: RepoSlot::FrontmatterFlattened },
            ],
        },
        assets: &[],
    };
}

impl SubstrateSchema<RepoSubstrate> for EmbeddedWorkflow {
    const SCHEMA: EntitySchema<RepoSlot> = EntitySchema {
        ref_asset: RefAssetDef {
            path_template: "{parent.base}/{id}/README.md",
            kind: &MARKDOWN_FILE,
            fields: &[
                FieldMapping { key: "name",        slot: RepoSlot::H1 },
                FieldMapping { key: "description", slot: RepoSlot::DescriptionParagraph },
                FieldMapping { key: "purpose",     slot: RepoSlot::Section("Purpose", SectionContent::Paragraph) },
                FieldMapping { key: "raci",        slot: RepoSlot::FrontmatterKey("raci") },
                FieldMapping { key: "steps",       slot: RepoSlot::FrontmatterKey("steps") },
                FieldMapping { key: "states",      slot: RepoSlot::FrontmatterKey("states") },
                FieldMapping { key: "intercepts",  slot: RepoSlot::FrontmatterKey("intercepts") },
                FieldMapping { key: "guidance",    slot: RepoSlot::Section("Guidance", SectionContent::Paragraph) },
                FieldMapping { key: "extensions",  slot: RepoSlot::FrontmatterFlattened },
            ],
        },
        assets: &[],
    };
}

impl SubstrateSchema<RepoSubstrate> for Task {
    const SCHEMA: EntitySchema<RepoSlot> = EntitySchema {
        ref_asset: RefAssetDef {
            path_template: "{parent.base}/{id}/README.md",
            kind: &MARKDOWN_FILE,
            fields: &[
                FieldMapping { key: "name",         slot: RepoSlot::H1 },
                FieldMapping { key: "description",  slot: RepoSlot::DescriptionParagraph },
                FieldMapping { key: "purpose",      slot: RepoSlot::Section("Purpose", SectionContent::Paragraph) },
                FieldMapping { key: "instructions", slot: RepoSlot::Section("Instructions", SectionContent::BulletList) },
                FieldMapping { key: "criteria",     slot: RepoSlot::Section("Criteria", SectionContent::BulletList) },
                FieldMapping { key: "artifact",     slot: RepoSlot::FrontmatterKey("artifact") },
                FieldMapping { key: "raci",         slot: RepoSlot::FrontmatterKey("raci") },
                FieldMapping { key: "states",       slot: RepoSlot::FrontmatterKey("states") },
                FieldMapping { key: "intercepts",   slot: RepoSlot::FrontmatterKey("intercepts") },
                FieldMapping { key: "guidance",     slot: RepoSlot::Section("Guidance", SectionContent::Paragraph) },
                FieldMapping { key: "extensions",   slot: RepoSlot::FrontmatterFlattened },
            ],
        },
        assets: &[AssetDef {
            path_template: "{parent.base}/{id}/template.md",
            kind: &RAW_FILE,
            fields: &[FieldMapping { key: "template_content", slot: RepoSlot::FileContent }],
            path_deps: &[],  // path depends only on entity_ref (id + parent)
        }],
    };
}

impl SubstrateSchema<RepoSubstrate> for Relay {
    const SCHEMA: EntitySchema<RepoSlot> = EntitySchema {
        ref_asset: RefAssetDef {
            path_template: "{parent.base}/{id}/README.md",
            kind: &MARKDOWN_FILE,
            fields: &[
                FieldMapping { key: "name",         slot: RepoSlot::H1 },
                FieldMapping { key: "description",  slot: RepoSlot::DescriptionParagraph },
                FieldMapping { key: "purpose",      slot: RepoSlot::Section("Purpose", SectionContent::Paragraph) },
                FieldMapping { key: "raci",         slot: RepoSlot::FrontmatterKey("raci") },
                FieldMapping { key: "delegates_to", slot: RepoSlot::FrontmatterKey("delegates_to") },
                FieldMapping { key: "briefing",     slot: RepoSlot::Section("Briefing", SectionContent::Paragraph) },
                FieldMapping { key: "debriefing",   slot: RepoSlot::Section("Debriefing", SectionContent::Paragraph) },
                FieldMapping { key: "state_map",    slot: RepoSlot::FrontmatterKey("state_map") },
                FieldMapping { key: "intercepts",   slot: RepoSlot::FrontmatterKey("intercepts") },
                FieldMapping { key: "guidance",     slot: RepoSlot::Section("Guidance", SectionContent::Paragraph) },
                FieldMapping { key: "extensions",   slot: RepoSlot::FrontmatterFlattened },
            ],
        },
        assets: &[],
    };
}
```

---

## TDD: Tests to Write First

```rust
// tests/repo_substrate.rs
use pari::substrate::repo::{RepoSubstrate, RepoCodec};
use pari::substrate::pipeline::Codec;
use pari::substrate::repo::slot::{RepoSlot, SectionContent};
use pari::substrate::pipeline::FieldMapping;
use std::collections::HashMap;
use tempfile::TempDir;

// --- RepoCodec decode/encode roundtrip ---

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
        FieldMapping { key: "name",        slot: RepoSlot::H1 },
        FieldMapping { key: "purpose",     slot: RepoSlot::FrontmatterKey("purpose") },
        FieldMapping { key: "extensions",  slot: RepoSlot::FrontmatterFlattened },
    ];
    let mut fields = HashMap::new();
    fields.insert("name",       serde_json::json!("Engineering Lead"));
    fields.insert("purpose",    serde_json::json!("Leads engineering"));
    fields.insert("extensions", serde_json::json!({ "x-owner": "alice" }));

    let encoded = codec.encode(&fields, schema).unwrap();
    let decoded = codec.decode(&encoded, schema).unwrap();

    assert_eq!(decoded.get("name").and_then(|v| v.as_str()), Some("Engineering Lead"));
    assert_eq!(decoded.get("purpose").and_then(|v| v.as_str()), Some("Leads engineering"));
    assert_eq!(decoded.get("extensions").and_then(|e| e.get("x-owner")).and_then(|v| v.as_str()), Some("alice"));
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

// --- RepoLocationResolver ---

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
    // parent.base is derived from the parent workflow id
    let entity = serde_json::json!({
        "entity_ref": { "id": "WriteProposal", "kind": "Task", "workflow_id": "InitiativeWorkflow" }
    });
    let path = resolver.resolve("{parent.base}/{id}/README.md", &entity);
    assert_eq!(path, root.join("workflows/InitiativeWorkflow/WriteProposal/README.md"));
}

// --- LCA computation ---

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

// --- RepoSubstrate integration ---

#[tokio::test]
async fn repo_substrate_write_and_read_role_roundtrip() {
    use pari::store::{EntityServer, EntityClient};
    use pari::entity::{AnyEntityRef, EntityRef};
    use pari::entities::role::{Role, TrackedRole};
    use std::collections::HashMap;

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
        EntityClient::insert(pari::entity::StoreEntity::from_role(TrackedRole::from(role))).await.unwrap();
        EntityClient::persist().await.unwrap();

        // Verify file exists
        let role_path = dir.path().join("roles/eng-lead.md");
        assert!(role_path.exists(), "Role file should be created on persist");

        // Read back — unload and re-resolve
        EntityClient::unload(AnyEntityRef::Role(EntityRef::new("eng-lead"))).await.unwrap();
        let resolved = EntityClient::resolve(AnyEntityRef::Role(EntityRef::new("eng-lead"))).await.unwrap();

        if let pari::entity::StoreEntity::Role(r) = resolved {
            // Load the name field
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
```

---

## Implementation Notes

### YAML Parsing

Use the `serde_yaml` crate for frontmatter parsing. Ensure it is in `Cargo.toml`:
```toml
[dependencies]
serde_yaml = "0.9"
```

### Markdown Format

The codec outputs a standard format:
1. YAML frontmatter block (`---` delimiters) — may be empty (`---\n---\n`)
2. Empty line
3. `# Name` H1 heading
4. Empty line
5. Optional description paragraph
6. One empty line per section separator
7. Each `## Section` heading followed by content

When decoding, sections are parsed by scanning for `## ` headings. The DescriptionParagraph is the content between the H1 line and the first `##` section (or end of file).

### Atomic Swap Details

The LCA swap uses two `fs::rename` calls:
1. `fs::rename(lca, lca.old)` — moves original directory aside
2. `fs::rename(lca.part, lca)` — moves staged directory into place

Both renames are atomic within the same filesystem. Hard-links (`fs::hard_link`) are used for unchanged files under LCA to avoid data copying.

### `{parent.base}` Resolution

For Task/Relay/EmbeddedWorkflow, the path template uses `{parent.base}`. The resolver resolves this by:
1. Looking up `entity["entity_ref"]["workflow_id"]` from the serialized entity
2. Constructing the parent Workflow's path: `workflows/{workflow_id}` (for Workflow parent type)
3. Computing `base_of` (parent directory): `workflows/{workflow_id}`

This is hardcoded as the Workflow directory convention. No recursive parent resolution needed.

---

## Acceptance Criteria

- `cargo test repo_substrate` passes — all tests green
- `RepoCodec` decode/encode roundtrip preserves all field types
- `FrontmatterFlattened` correctly captures `x-*` extension keys
- `BulletList` section decodes to `Vec<String>`
- `compute_lca` finds the correct LCA for sibling and nested file paths
- `RepoSubstrate::new` removes stale `.part` and `.old` directories
- `RepoSubstrate` write + read roundtrip produces the correct entity from disk
- Entity path templates resolve correctly: Role → `roles/{id}.md`, Task → `{parent.base}/{id}/README.md`
- Tasks 01-10 tests still pass
