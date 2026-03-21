## 1. Tracked<T> Primitive

- [ ] 1.1 Write tests for Tracked<T>: new starts dirty, Deref reads transparently, DerefMut marks dirty, is_dirty/reset_dirty lifecycle
- [ ] 1.2 Implement Tracked<T> in src/tracked.rs: newtype with Deref, DerefMut, is_dirty, reset_dirty

## 2. TrackedMap<K,V> Primitive

- [ ] 2.1 Add indexmap dependency to Cargo.toml
- [ ] 2.2 Write tests for TrackedMap: insert marks dirty, remove records in removed set, get_mut marks dirty, insertion order preserved, shift_remove preserves order
- [ ] 2.3 Implement TrackedMap<K,V> in src/tracked.rs: IndexMap-backed with dirty set, removed set, insert/remove/get/get_mut/keys/values
- [ ] 2.4 Write tests for TrackedMap::drain_changes: returns dirty + removed sets, resets tracking state
- [ ] 2.5 Implement TrackedMap::drain_changes
- [ ] 2.6 Write tests for TrackedMap::from_vec: preserves order, extracts keys, entries start dirty
- [ ] 2.7 Implement TrackedMap::from_vec

## 3. Derive Macro

- [ ] 3.1 Create pari-macros crate with proc-macro Cargo.toml
- [ ] 3.2 Write tests for derive on a simple flat struct (all fields become Tracked<T>, From impl works)
- [ ] 3.3 Implement #[derive(Tracked)] for flat structs: generates tracked variant struct + From impl
- [ ] 3.4 Write tests for derive with #[tracked(map_key = "id")] annotation on Vec fields
- [ ] 3.5 Implement #[tracked(map_key)] support: Vec<T> field becomes TrackedMap<String, TrackedT> with from_vec conversion
- [ ] 3.6 Write tests for derive on nested structs (field whose type is itself a tracked entity)
- [ ] 3.7 Implement nested tracked type support in the derive macro

## 4. Apply Derive to Entity Structs

- [ ] 4.1 Add #[derive(Tracked)] to Role, generate TrackedRole
- [ ] 4.2 Add #[derive(Tracked)] to Hook, generate TrackedHook
- [ ] 4.3 Add #[derive(Tracked)] to Team, generate TrackedTeam
- [ ] 4.4 Add #[derive(Tracked)] to Task and Relay (embedded entities)
- [ ] 4.5 Add #[derive(Tracked)] to ReviewStep, WorkStep, SharedWorkStep and step enum variant structs
- [ ] 4.6 Add #[derive(Tracked)] to Workflow and SharedWorkflow with #[tracked(map_key = "id")] on steps field
- [ ] 4.7 Verify all tracked variants compile and From impls work end-to-end with a nested workflow

## 5. EntityStore Internals

- [ ] 5.1 Write tests for EntityStore insertion API: insert_role/hook/team/workflow/shared_workflow accept plain types
- [ ] 5.2 Change EntityStore internals from HashMap<String, Entity> to TrackedMap<String, TrackedEntity>
- [ ] 5.3 Implement typed insertion methods (insert_role, insert_hook, etc.) that convert plain to tracked
- [ ] 5.4 Write tests for EntityStore read access: has_* return bool, get_* return tracked instances (e.g., &TrackedRole) with fields accessible via Deref
- [ ] 5.5 Update has_*/get_* methods to work through TrackedMap, returning tracked instances
- [ ] 5.6 Write tests for EntityStore mutable access: get_*_mut returns tracked references, mutations mark dirty
- [ ] 5.7 Implement get_*_mut and remove_* methods
- [ ] 5.8 Update existing EntityStore tests for new insertion API

## 6. ChangeSet and drain_changes

- [ ] 6.1 Write tests for EntityKind, ChangeOp, EntityChange, and ChangeSet types
- [ ] 6.2 Define ChangeSet, EntityChange, ChangeOp, EntityKind, EntityData types in src/substrate/changeset.rs
- [ ] 6.3 Write tests for drain_changes: collects added/modified/removed across all entity types, resets dirty state, walks nested workflow steps producing flat entries with paths
- [ ] 6.4 Implement EntityStore::drain_changes — walk TrackedMaps, collect flat EntityChange entries with paths, reset dirty flags

## 7. Substrate Trait Update

- [ ] 7.1 Change Substrate::persist signature from &EntityStore to &ChangeSet
- [ ] 7.2 Update all trait implementors and call sites for new signature

## 8. RepoSubstrate Incremental Persist

- [ ] 8.1 Write tests for LCA computation: single path returns parent, sibling paths return common parent, cross-top-level paths return root
- [ ] 8.2 Implement LCA computation utility from a set of file paths
- [ ] 8.3 Write tests for incremental persist: single entity change swaps only parent dir, empty changeset is no-op, initial persist with no existing root creates everything
- [ ] 8.4 Implement RepoSubstrate::persist with LCA-based staging: hard-link unchanged siblings, write changed files, atomic directory swap
- [ ] 8.5 Write tests for hard-link fallback to copy on cross-device error
- [ ] 8.6 Implement cross-device fallback in hard-link logic
- [ ] 8.7 Write tests for batch atomicity: crash-safe staging (stale .part/.old cleanup)
- [ ] 8.8 Implement stale .part/ and .old/ cleanup on startup

## 9. Integration

- [ ] 9.1 Update existing storage integration tests for new persist(&ChangeSet) signature
- [ ] 9.2 Write end-to-end test: create store, insert entities, drain_changes, persist, verify files on disk
- [ ] 9.3 Write end-to-end test: modify one field, drain_changes, persist, verify only affected subtree changed
