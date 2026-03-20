## Context

The current entity layer (`src/schema/`) defines six entity types with a flat model where Task and Relay are top-level entities. Steps in a Workflow reference tasks and relays by id; the actual entity definitions live separately. There is no persistence layer — entities exist only in memory as Rust structs.

This change introduces two related breaks:
1. **Structural**: WorkStep embeds its definition inline (Task/Relay/Workflow). Task and Relay become embedded-only. A `WorkflowDef<S>` generic enables `Workflow` and `SharedWorkflow` to share structure. An `Extensions` newtype supports user-defined `x-` prefixed fields on all entities.
2. **Persistence**: A new `LocalFileStorage` struct writes validated entities to a caller-provided directory as markdown + YAML frontmatter files. `persist()` is implemented now; `load()` is the next phase.

Constraints: schemars 0.8.x; no runtime; no parser yet (deferred).

## Goals / Non-Goals

**Goals:**
- Restructure `WorkStep` to embed `WorkStepDefinition` inline; derive step identity from inner entity id
- Add `WorkflowDef<S>` generic; introduce `SharedWorkflow = WorkflowDef<SharedStep>`
- Add `Extensions` newtype to all entity structs; validate `x-` key format at all levels
- Update `RepoContext` with typed workflow collections (`workflows: Vec<Workflow>`, `shared_workflows: Vec<SharedWorkflow>`)
- Compose validators: structural validation of full entity tree first, then semantic across full tree
- Add `src/storage/repo/`: `RepoStorage` in `repo/storage.rs` implementing a generic `Storage<D>` trait with atomic `persist()`; render functions in `repo/render.rs`
- Update xtask: remove `task.json` / `relay.json`; regenerate all schemas with `patternProperties` / `additionalProperties: false`
- Promote `serde_json` to a regular dependency; add `serde_yaml`

**Non-Goals:**
- `load()` / reading entities from disk (next phase for storage; also parser proposal)
- `pari.yaml` config parsing — root path is always passed by the caller
- Versioning or migration of stored entity formats

## Decisions

### 1. WorkflowDef<S> generic

`WorkflowDef<S>` is a generic struct with `steps: Vec<S>`. `Workflow = WorkflowDef<Step>`, `SharedWorkflow = WorkflowDef<SharedStep>`. `S` is bounded by `JsonSchema + Serialize + DeserializeOwned`. schemars 0.8 supports generic derives with these bounds.

**Alternative considered**: Two separate structs. Rejected — identical fields except `steps`; generic avoids duplication.

### 2. WorkStep embeds definition; step id from inner entity

New shape: `WorkStep { depends_on: Option<Vec<String>>, definition: WorkStepDefinition }`.

`WorkStepDefinition` is an untagged serde enum: `Task | Relay | Box<Workflow>`. Untagged discrimination works because each variant has a distinct required field (`artifact` → Task, `delegates_to` → Relay, `steps` → Workflow).

`Step::id()` delegates to `definition.id()` via an `id()` method on `WorkStepDefinition`. `ReviewStep` shape is unchanged.

Parallel for shared: `SharedWorkStep { depends_on, definition: SharedWorkStepDefinition }` where `SharedWorkStepDefinition = Task | Box<SharedWorkflow>` — Relay is excluded from shared workflows.

**Alternative considered**: Explicit `type:` discriminator field. Rejected — unique required fields already disambiguate; a discriminator adds redundancy.

### 3. Validation phases and composition

Validation runs in two ordered phases over the entire entity tree:

**Phase 1 — structural**: Walk the full tree (workflow → embedded steps → embedded tasks/relays/sub-workflows → their nested fields). Collect all id format errors, missing required fields, and min-count violations. Extensions are structurally valid as long as keys match `^x-` (format-only check in this phase).

**Phase 2 — semantic**: Cross-field constraints (ReviewStep ordering, depends_on resolution, state semantic completeness, RACI + hook referential integrity). Run only if Phase 1 produced no errors, since structural failures can cause confusing secondary semantic errors.

Errors from embedded entity validators are path-prefixed via `prefix_errors(errors, prefix)` before collection:
```
validate_workflow(wf, ctx):
  Phase 1: validate_structure_tree (self + recursive children)
  if errors.is_empty():
    Phase 2: validate_semantic_tree (self + recursive children, with ctx)
```

`validate_extensions(extensions: &Extensions, path: &str) -> Vec<ValidationError>` lives in `validation.rs` and is called by every entity validator during Phase 1.

### 4. Extensions

`Extensions` is `pub struct Extensions(pub HashMap<String, serde_json::Value>)`. All entity structs gain `#[serde(flatten)] pub extensions: Extensions`. `validate_extensions` checks every key matches `^x-` and emits a `ValidationError` per violation (Phase 1).

For JSON Schema: verified by test — schemars 0.8 correctly emits `patternProperties: { "^x-": true }` when `Extensions` is flattened into a struct. No xtask post-processing required.

### 5. Storage trait and RepoStorage interface

`EntityStore` is a neutral container of all validated entity collections — not tied to any storage implementation. `Storage` takes `&EntityStore`; what varies across implementations is the target (filesystem, database, etc.), not what is stored.

```rust
// src/storage/mod.rs — neutral, no "repo" concepts here
pub struct EntityStore {
    pub roles: Vec<Role>,
    pub hooks: Vec<Hook>,
    pub teams: Vec<Team>,
    pub workflows: Vec<Workflow>,
    pub shared_workflows: Vec<SharedWorkflow>,
}

pub trait Storage {
    fn persist(&self, store: &EntityStore) -> Result<(), Vec<StorageError>>;
}

pub struct StorageError {
    pub path: String,
    pub message: String,
}
```

`RepoStorage` is the repository-specific implementation — "Repo" is appropriate here and only here. Render functions are specific to the repository file format and co-located in `repo/`:

```rust
// src/storage/repo/storage.rs
pub struct RepoStorage { root: PathBuf }
impl RepoStorage { pub fn new(root: impl Into<PathBuf>) -> Self }
impl Storage for RepoStorage { ... }   // persists EntityStore to a directory tree

// src/storage/repo/render.rs — repo file format (YAML frontmatter + markdown body)
pub fn render_role(role: &Role) -> String { ... }
// ... one render fn per entity type
```

Module layout:
```
src/storage/
  mod.rs         — EntityStore, Storage trait, StorageError; pub mod repo
  repo/
    mod.rs        — pub mod storage; pub(crate) mod render
    storage.rs    — RepoStorage, impl Storage for RepoStorage
    render.rs     — render_role, render_hook, render_team,
                    render_workflow_readme, render_task_readme, render_relay_readme
```

`RepoStorage::new` accepts any path — no `.pari/` default is hardcoded.

**`persist()` is all-or-nothing**: writes to a sibling `<dirname>.part/` temp directory, then renames the entire directory to the target. On any error, the temp directory is deleted and no partial state appears at the target path:

```
persist():
  temp = root.parent() / "<root_dirname>.part"
  try:
    write all entity files under temp/
    rename(temp, root)
  catch:
    remove_dir_all(temp)   // clean up
    return errors
```

`rename()` on the same filesystem is atomic on POSIX. The `.part` suffix signals work-in-progress.

Per-entity `render(entity) -> String` functions emit YAML frontmatter + markdown body per the field-to-section mapping in the proposal.

### 6. Schema generation: remove task.json and relay.json

Task and Relay are embedded-only; `task.json` and `relay.json` are removed from `schemas/`. The xtask no longer calls `write_schema::<Task>()` or `write_schema::<Relay>()`. Their types appear as `$defs` within `workflow.json` via schemars' automatic inlining.

### 7. Dependency changes

`serde_json` promoted from dev-dependency to regular dependency (required by `Extensions` using `serde_json::Value`). `serde_yaml` added as a regular dependency for the storage serializer.

## Risks / Trade-offs

- **schemars patternProperties** → Resolved: schemars 0.8 emits `patternProperties` natively for flattened `Extensions`. No xtask post-processing needed.
- **High test churn** from WorkStep breaking change → Tasks co-locate test updates with implementation in lockstep
- **Untagged enum ambiguity** for WorkStepDefinition → Low risk; unique required fields per variant; runtime validation catches malformed input
- **`rename()` cross-device** → Write temp in same parent directory as target to guarantee same filesystem

## Open Questions

None — scope is fully defined.
