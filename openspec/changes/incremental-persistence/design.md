## Context

The substrate layer currently takes a full `EntityStore` snapshot and writes every entity to disk on each `persist()` call. The `RepoSubstrate` implementation writes all files to a `.part/` staging directory, then atomically renames it to the root. This works but is O(total entities) regardless of how many actually changed.

The codebase has ~5 entity types (Role, Hook, Team, SharedWorkflow, Workflow), each with multiple fields. Workflows contain nested step trees (Tasks, Relays, inline Workflows). A single field edit on a deeply nested Task currently triggers a full re-render and re-write of every entity in the store.

## Goals / Non-Goals

**Goals:**
- Track field-level changes across all entity types transparently — callers construct and mutate plain structs as today
- Produce a substrate-agnostic `ChangeSet` describing exactly what changed (entity-level ops + dirty field names)
- Make `persist()` incremental — only re-render and write entities that actually changed
- Preserve batch atomicity in RepoSubstrate — a crash mid-persist leaves the repo in a consistent state
- Keep plain entity structs unchanged — no impact on serde, JSON schema generation, or validation

**Non-Goals:**
- `load()` / Read capability — deferred to a subsequent proposal
- Delete capability — deferred to a subsequent proposal
- Undo/rollback of in-memory changes
- Conflict detection or merge between concurrent writers

## Decisions

### 1. Deep pervasive `Tracked<T>` newtype for field-level change tracking

Every field on every entity gets wrapped in `Tracked<T>` in the tracked variant. `Tracked<T>` implements `Deref` (transparent reads) and `DerefMut` (marks dirty on mutable access). This gives automatic field-level dirty tracking without requiring callers to use special mutation methods.

**Alternatives considered:**
- *Entity-level dirty flag on EntityStore*: Knows an entity changed but not which fields — insufficient for substrates that persist at field granularity (databases, APIs).
- *Snapshot diff at flush time*: Requires `Clone + PartialEq` on all entities, O(total) comparison cost, and doesn't track deletions.
- *Explicit `set()` methods only (no DerefMut)*: Eliminates false positives from `&mut` borrows that don't change values, but forces a non-idiomatic mutation API. DerefMut false positives are acceptable — the cost is an unnecessary re-render of one entity, not correctness.

### 2. `TrackedMap<K,V>` backed by IndexMap for collections

`TrackedMap` wraps `IndexMap<K,V>` and tracks three sets: `dirty` (inserted or modified keys), `removed` (deleted keys), and per-value dirty flags via `Tracked<V>`. IndexMap preserves insertion order, which is critical for workflow steps where execution sequence matters.

Used in two places:
- **EntityStore**: `TrackedMap<String, TrackedRole>`, `TrackedMap<String, TrackedWorkflow>`, etc.
- **Workflow steps** (internal): `TrackedMap<String, TrackedStep>` keyed by step id.

`Vec<Step>` remains in the plain structs. The `From<Workflow> for TrackedWorkflow` conversion extracts step ids and builds the IndexMap. Serde continues to target the plain `Vec<Step>` — no custom serialization needed.

**Alternatives considered:**
- *HashMap*: Loses insertion order. Step ordering is critical.
- *BTreeMap*: Sorted by key, not insertion order. Wrong semantics.
- *Vec with index tracking*: Detecting additions/removals requires diffing or a manifest — pushes substrate-level bookkeeping into the entity model.

### 3. Plain structs stay unchanged — tracked variants are derived

Plain entity structs (`Role`, `Task`, `Workflow`, etc.) remain the public API, serde target, and JSON schema source. A `#[derive(Tracked)]` proc macro generates:
- A tracked struct variant (e.g., `TrackedRole`) with each field wrapped in `Tracked<T>`
- A `From<Plain> for Tracked` impl that wraps each field

Callers construct plain structs and insert them into EntityStore. The store boundary converts to tracked variants internally. Reading through `Deref` returns plain field references. `Tracked` never appears in public API signatures.

**Alternatives considered:**
- *Single struct with Tracked fields + field-level `From` impls*: Rust's `From`/`Into` doesn't chain (`&str → RoleId → Tracked<RoleId>` fails), making construction inconsistent across field types. Tracked also leaks into serde, schema generation, and validation signatures.
- *Manual tracked struct definitions*: Correct but requires maintaining parallel struct definitions by hand. The derive macro eliminates this boilerplate.

### 4. Flat `ChangeSet` with path-based entries

`EntityStore::drain_changes()` walks the tracked tree and produces a flat list of `EntityChange` entries. Each entry carries:
- `path`: tree location (e.g., `"workflows/Initiative/WriteProposal"`)
- `kind`: entity kind (Role, Task, Workflow, etc.)
- `id`: entity id
- `op`: `Added(entity)`, `Modified { entity, dirty_fields }`, or `Removed`

`Modified` carries the full plain entity (for re-rendering) plus a `Vec<String>` of dirty field names (for substrates that can do targeted updates). The `drain_changes()` call resets all dirty flags.

**Alternatives considered:**
- *Nested changeset mirroring entity tree*: Dense, harder for substrates to consume. Each substrate would need to traverse the nested structure to find what it cares about.
- *ChangeSet carries Tracked entities instead of plain*: Leaks `Tracked` past the drain boundary. Substrates should not depend on the tracking mechanism.
- *Single top-level entity changes with deep dirty paths*: Paths like `"steps[WriteProposal].instructions"` require parsing. Flat entries with a path prefix are simpler and directly mappable to filesystem locations or DB table paths.

### 5. `persist()` signature changes to accept `&ChangeSet`

The `Substrate::persist()` method changes from `persist(&self, store: &EntityStore)` to `persist(&self, changeset: &ChangeSet)`. The substrate receives a pre-built changeset and does not interact with EntityStore directly. This cleanly separates change detection (EntityStore's responsibility) from persistence (Substrate's responsibility).

### 6. LCA-based atomic persistence for RepoSubstrate

RepoSubstrate computes the lowest common ancestor (LCA) directory of all changed file paths. It stages changes within only that subtree:

1. Create `.part/` sibling of the LCA directory
2. Hard-link unchanged files within the LCA subtree into `.part/`
3. Write changed files into `.part/`
4. Omit removed files from `.part/`
5. Rename LCA directory → `.old/`, rename `.part/` → LCA directory, delete `.old/`

Cost is O(files under LCA), not O(total repo). Degrades gracefully: scattered changes push the LCA toward root, approaching full-snapshot cost. Single-entity changes swap only the entity's immediate parent directory.

For the initial persist (no existing root), the LCA is root — identical to the current full-snapshot behavior.

### 7. Proc-macro crate `pari-macros`

The `#[derive(Tracked)]` macro lives in a separate `pari-macros` crate (Rust requires proc macros in their own crate). The macro:
- Reads the plain struct's fields and types
- Generates a `Tracked<StructName>` struct with each field wrapped in `Tracked<T>`
- Generates `From<Plain> for Tracked` conversion
- Handles nested tracked types: if a field's type is itself a tracked entity, the conversion recurses
- Handles `Vec<Step>` → `TrackedMap<String, TrackedStep>` conversion for step fields (annotated with `#[tracked(map_key = "id")]`)

## Risks / Trade-offs

- **DerefMut false positives** — Any `&mut` borrow marks the field dirty, even if the value doesn't change. This may cause unnecessary re-renders. Acceptable for RepoSubstrate (rendering is cheap); may warrant a `set_if_changed()` method for expensive substrates later. → *Mitigation: document the behavior; add `set_if_changed()` if profiling shows waste.*

- **Proc-macro complexity** — Proc macros are harder to debug and test than regular code. The macro must handle nested types, optional fields, and special annotations. → *Mitigation: start with manual impls for 1-2 entities to stabilize the pattern, then extract the macro.*

- **Hard-link assumptions** — Hard-linking requires source and target on the same filesystem. Cross-filesystem scenarios (e.g., root on a mounted volume) would fail. → *Mitigation: fall back to file copy if `hard_link()` returns `EXDEV`.*

- **LCA can degrade to full snapshot** — Changes across unrelated top-level directories (e.g., a role and a workflow) push the LCA to root, making the atomic swap equivalent to a full snapshot. → *Acceptable: this is the worst case, not the common case. No regression from current behavior.*

- **TrackedMap ordering on removal** — `IndexMap::shift_remove()` is O(n) to preserve insertion order. → *Acceptable: step counts per workflow are small (typically < 20). swap_remove would be O(1) but loses ordering.*
