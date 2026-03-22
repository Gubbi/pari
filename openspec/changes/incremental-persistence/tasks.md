## 1. Tracked<T> Primitive

- [ ] 1.1 Write tests for Tracked<T>: new starts dirty, Deref reads transparently, DerefMut marks dirty, is_dirty/reset_dirty lifecycle
- [ ] 1.2 Implement Tracked<T> in src/tracked.rs: newtype with Deref, DerefMut, is_dirty, reset_dirty

## 2. TrackedMap<K,V> Primitive

- [ ] 2.1 Add indexmap dependency to Cargo.toml
- [ ] 2.2 Write tests for TrackedMap: insert records key in inserted set, remove retains full value in removed map, get_mut records key in modified set, inserted takes precedence over modified, insertion order preserved, shift_remove preserves order
- [ ] 2.3 Implement TrackedMap<K,V> in src/tracked.rs: IndexMap-backed with inserted/modified sets and removed: IndexMap<K,V>, insert/remove/get/get_mut/iter_mut/keys/values
- [ ] 2.4 Write tests for TrackedMap::has_changes and reset_tracked: has_changes returns true when sets non-empty, reset_tracked clears all sets and drops removed values
- [ ] 2.5 Implement TrackedMap::has_changes and reset_tracked
- [ ] 2.6 Write tests for TrackedMap::from_vec: preserves order, entries land in inserted set
- [ ] 2.7 Implement TrackedMap::from_vec

## 3. Derive Macro

- [ ] 3.1 Create pari-macros crate with proc-macro Cargo.toml
- [ ] 3.2 Write tests for derive on a simple flat struct: all fields become Tracked<T>, From impl works, dirty_fields() returns names of dirty fields only
- [ ] 3.3 Implement #[derive(Tracked)] for flat structs: generates tracked struct + From impl + dirty_fields() method
- [ ] 3.4 Write tests for derive on enums: Tracked prefix applied to variant inner types, From impl matches on variants, dirty_fields() delegates to active variant
- [ ] 3.5 Implement #[derive(Tracked)] for enums
- [ ] 3.6 Write tests for derive on generic structs: TrackedWorkflowDef<TS> generated, From<WorkflowDef<S>> for TrackedWorkflowDef<TS> where TS: From<S>
- [ ] 3.7 Implement #[derive(Tracked)] for generic structs: preserve type params, introduce TS: From<S> bounds
- [ ] 3.8 Write tests for #[tracked(map_key = "id")] annotation: Vec<S> field becomes TrackedMap<String, TS>, from_vec conversion via TS::from
- [ ] 3.9 Implement #[tracked(map_key)] support

## 4. Apply Derive to Entity Structs

- [ ] 4.1 Add #[derive(Tracked)] to Role, generate TrackedRole
- [ ] 4.2 Add #[derive(Tracked)] to Hook, generate TrackedHook
- [ ] 4.3 Add #[derive(Tracked)] to Team, generate TrackedTeam
- [ ] 4.4 Add #[derive(Tracked)] to Task and Relay (embedded entities)
- [ ] 4.5 Add #[derive(Tracked)] to ReviewStep, WorkStep<S>, SharedWorkStep<S>
- [ ] 4.6 Add #[derive(Tracked)] to WorkStepDefinition and SharedWorkStepDefinition (enum derive)
- [ ] 4.7 Add #[derive(Tracked)] to Step<S> and SharedStep<S> (generic enum derive)
- [ ] 4.8 Add #[derive(Tracked)] to WorkflowDef<S> with #[tracked(map_key = "id")] on steps field; declare TrackedWorkflow and TrackedSharedWorkflow type aliases manually
- [ ] 4.9 Verify all tracked variants compile and From impls work end-to-end with a nested workflow

## 5. EntityStore Internals

- [ ] 5.1 Write tests for EntityStore insertion API: insert_role/hook/team/workflow/shared_workflow accept plain types, insertions appear in TrackedMap inserted set
- [ ] 5.2 Change EntityStore internals from HashMap<String, Entity> to TrackedMap<String, TrackedEntity>
- [ ] 5.3 Implement typed insertion methods (insert_role, insert_hook, etc.) that convert plain to tracked
- [ ] 5.4 Write tests for EntityStore read access: has_* return bool, get_* return tracked instances with fields accessible via Deref
- [ ] 5.5 Update has_*/get_* methods to work through TrackedMap
- [ ] 5.6 Write tests for EntityStore mutable access: get_*_mut returns tracked references, mutations mark field dirty and record key in modified set
- [ ] 5.7 Implement get_*_mut and remove_* methods
- [ ] 5.8 Update existing EntityStore tests for new insertion API

## 6. ChangeSet and collect_changes

- [ ] 6.1 Write tests for EntityKind, ChangeOp, EntityChange, and ChangeSet types; EntityData carries tracked entity types
- [ ] 6.2 Define ChangeSet, EntityChange, ChangeOp, EntityKind, EntityData types in src/substrate/changeset.rs
- [ ] 6.3 Write tests for collect_changes: collects added/modified/removed across all entity types, does NOT reset dirty state, dirty_fields populated correctly via dirty_fields() method, walks nested workflow steps producing flat entries with paths
- [ ] 6.4 Implement EntityStore::collect_changes(&self) — walk TrackedMaps, collect flat EntityChange entries with paths, call dirty_fields() on modified entities
- [ ] 6.5 Write tests for EntityStore::reset_tracked: clears all tracking state across all maps including nested step maps, subsequent collect_changes returns empty ChangeSet
- [ ] 6.6 Implement EntityStore::reset_tracked(&mut self)

## 7. Substrate Trait Update

- [ ] 7.1 Rename Substrate::persist to atomic_persist; change signature from &EntityStore to &ChangeSet
- [ ] 7.2 Update all trait implementors and call sites for new signature

## 8. RepoSubstrate Incremental Persist

- [ ] 8.1 Write tests for LCA computation: single path returns parent, sibling paths return common parent, cross-top-level paths return root
- [ ] 8.2 Implement LCA computation utility from a set of file paths
- [ ] 8.3 Write tests for incremental persist: single entity change swaps only parent dir, empty changeset is no-op, initial persist with no existing root creates everything
- [ ] 8.4 Implement RepoSubstrate::atomic_persist with LCA-based staging: hard-link unchanged siblings, write changed files, atomic directory swap
- [ ] 8.5 Write tests for hard-link fallback to copy on cross-device error
- [ ] 8.6 Implement cross-device fallback in hard-link logic
- [ ] 8.7 Write tests for batch atomicity: crash-safe staging (stale .part/.old cleanup)
- [ ] 8.8 Implement stale .part/ and .old/ cleanup on startup

## 9. Integration

- [ ] 9.1 Update existing storage integration tests for new atomic_persist(&ChangeSet) signature
- [ ] 9.2 Write end-to-end test: create store, insert entities, collect_changes, atomic_persist, reset_tracked, verify files on disk
- [ ] 9.3 Write end-to-end test: modify one field, collect_changes, atomic_persist, reset_tracked, verify only affected subtree changed
- [ ] 9.4 Write end-to-end test: atomic_persist fails mid-write, verify reset_tracked NOT called, verify collect_changes still returns the same ChangeSet
