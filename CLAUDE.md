# Pari — Codebase Guide

## What This Is

Rust library (`pari`) — a workflow runtime for hybrid human-agent teams. Two top-level modules:

- `src/schema/` — entity types, validation, entity store
- `src/substrate/` — persistence backend trait + repo (filesystem) implementation

---

## Module Map

```
src/
  schema/
    entities/    Role, Hook, Team, Workflow, SharedWorkflow
                 Task, Relay — embedded-only (no standalone entity, no top-level schema)
    store.rs     EntityStore — TrackedMap collections keyed by id; dual-purpose: validation context + persist input
    types.rs     Shared types (Raci, Artifact, HookInvocation, state types, Extensions, ...)
    validation.rs  validate() implementations per entity
  substrate/
    mod.rs       Substrate trait (atomic_persist only; load deferred to future proposal), SubstrateError
    changeset.rs ChangeSet, EntityChange, ChangeOp, EntityKind, EntityData
    repo/
      storage.rs   RepoSubstrate — LCA-based atomic swap via .part//.old/ dirs
      lca.rs       LCA computation over file paths
      render.rs    Markdown+YAML-frontmatter renderers per entity type
  tracked.rs     Tracked<T>, TrackedMap<K,V>, HasId, #[derive(Tracked)] macro (pari-macros crate)
  lib.rs         Module declarations only — no re-exports
```

---

## Entity Type Reference

### Standalone entities (top-level, have their own file on disk)

```rust
Role       { id: RoleId (kebab), name, purpose, traits?: Vec<String>, ..extensions }
Hook       { id: HookId (kebab), name, description, inputs?: Vec<HookInput>, ..extensions }
           HookInput { name, description }
Team       { id: TeamId (kebab), name, description?, members?: Vec<TeamMember>,
             include?: HashMap<String,String>, import?: Vec<String>, ..extensions }
           TeamMember { handle, role }
Workflow   = WorkflowDef<WorkStepDefinition>
SharedWorkflow = WorkflowDef<SharedWorkStepDefinition>
WorkflowDef<S> { id: WorkflowId (CamelCase), name, description?, purpose,
                 accountability: Raci, steps: Vec<Step<S>>,
                 states: Vec<WorkflowStateEntry>, hooks?: HooksMap,
                 guidance?, ..extensions }
```

### Embedded-only (live inside workflow steps, no top-level files)

```rust
Task   { id: TaskId (CamelCase), name, description?, purpose, instructions: Vec<String>,
         criteria: Vec<String>, accountability?: Raci, artifact: Artifact,
         states: Vec<TaskStateEntry>, hooks?: HooksMap, guidance?, ..extensions }
Relay  { id: RelayId (CamelCase), name, description?, purpose, accountability?: Raci,
         delegates_to: String, briefing?, debriefing?,
         state_map: HashMap<String,StateMapEntry>, hooks?: HooksMap, guidance?, ..extensions }
```

### Step type hierarchy

```rust
Step<S>          = Work(WorkStep<S>) | Review(ReviewStep)   // serde untagged
WorkStep<S>      { depends_on?: Vec<String>, definition: S }
ReviewStep       { id: String (CamelCase), approver: String, on_reject: String }
WorkStepDefinition       = Task(Task) | Relay(Relay) | Workflow(Box<Workflow>)
SharedWorkStepDefinition = Task(Task) | SharedWorkflow(Box<SharedWorkflow>)  // no Relay
```

### EntityStore

```rust
EntityStore {
    roles:            TrackedMap<String, TrackedRole>,
    hooks:            TrackedMap<String, TrackedHook>,
    teams:            TrackedMap<String, TrackedTeam>,
    workflows:        TrackedMap<String, TrackedWorkflow>,
    shared_workflows: TrackedMap<String, TrackedSharedWorkflow>,
}
// API: insert_role/hook/team/workflow/shared_workflow (plain → tracked)
//      has_*/get_*/get_*_mut/remove_*
//      collect_changes(&self) -> ChangeSet  — does NOT reset state
//      reset_tracked(&mut self)
```

### Change tracking primitives

```rust
Tracked<T>          // newtype; Deref/DerefMut; is_dirty(), reset_dirty()
TrackedMap<K,V>     // IndexMap-backed; tracks inserted/modified/removed sets
                    // insert/remove/get/get_mut/iter_mut/keys/values/len/is_empty
                    // has_changes(), reset_tracked(), from_vec()
#[derive(Tracked)]  // generates Tracked* struct, From impl, dirty_fields() method
                    // #[tracked(map_key = "id")] on Vec<S> field → TrackedMap
```

### ChangeSet types

```rust
ChangeSet<'a>   { changes: Vec<EntityChange<'a>> }
EntityChange<'a>{ path: String, kind: EntityKind, id: String, op: ChangeOp<'a> }
EntityKind      = Role | Hook | Team | Workflow | SharedWorkflow
ChangeOp<'a>    = Added(&'a TrackedEntity) | Modified { entity, dirty_fields: Vec<String> }
                | Removed(String)  // id
```

### Substrate trait

```rust
trait Substrate {
    fn atomic_persist(&self, changeset: &ChangeSet<'_>) -> Result<(), Vec<SubstrateError>>;
}
SubstrateError { path: String, message: String }
// RepoSubstrate: LCA of changed paths → stage in .part/, swap via fs::rename
// Startup: cleans up stale .part/ and .old/ dirs
```

---

## Key Decisions

**EntityStore invariant**: the entity being validated must NOT already be in the store. Callers enforce this.

**Task and Relay are embedded-only**: they live inside workflow steps, not as top-level entities. No standalone schema is generated for them.

**Atomic persistence**: RepoSubstrate computes LCA of all changed file paths, stages changes in `<lca>.part/`, hard-links unchanged siblings, then swaps via `fs::rename`. Errors collected, not short-circuited. Stale `.part/` and `.old/` dirs cleaned on startup.

**Extensions pattern**: every entity has an `extensions: Extensions` field (`HashMap<String, serde_json::Value>`) via `#[serde(flatten)]`. Only `x-` prefixed keys are allowed by schema.

**Schema generation**: `cargo xtask` drives `schemars` codegen into `schemas/`. Post-processing step adds `additionalProperties: false` to schemas with `patternProperties` (schemars 0.8 limitation with `#[serde(flatten)]`).

**Substrate::load is not yet defined**: the trait currently has `atomic_persist()` only. Loading from a substrate is a future proposal.

**collect_changes does not reset state**: callers must explicitly call `reset_tracked()` after a successful persist. This preserves the changeset for retry on failure.

---

## Running Things

```sh
cargo test               # all tests (inline unit + schema coherence + storage integration)
cargo xtask              # regenerate schemas/ from Rust types
```

Tests: ~269 total across unit (inline), `tests/schema_coherence.rs`, `tests/storage_integration.rs`.

---

## Conventions

- IDs: kebab-case for Role/Team/Hook (e.g. `eng-lead`), CamelCase for Workflow/Task/Relay (e.g. `InitiativeWorkflow`)
- Inline unit tests in `#[cfg(test)]` blocks within each source file
- Integration tests in `tests/`
- No `pub use` re-exports at crate root — callers use full paths (`pari::schema::entities::role::Role`)
