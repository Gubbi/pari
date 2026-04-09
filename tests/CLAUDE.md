# tests/ — Integration Test Files

All tests use the full public API (`pari::...`). No mocking of store internals.

---

## Test Files

### `core_jobs.rs`
8 end-to-end tests for the core jobs the library supports. Full stack: `EntityClient` → `EntityServer` → `Store<RepoSubstrate>` → filesystem. Uses `TempDir`.

| Test | Job |
|------|-----|
| `job_1_read_entity` | Load entity written by a prior session |
| `job_2_define_new_entity` | Insert + persist creates file on disk |
| `job_3_update_entity` | checkout → mutate → commit → persist overwrites file |
| `job_4_remove_entity` | resolve → remove → persist deletes file |
| `job_5_save_all_pending_changes` | Single persist flushes add + update + remove |
| `job_6_abandon_in_progress_edit` | undo_checkout discards changes, releases lock |
| `job_7_rollback_staged_change` | undo_commit reverts committed-but-not-persisted change |
| `job_8_refresh_from_substrate` | unload + resolve re-reads changed file from disk |

### `store_operations.rs`
Unit-level store operations using `InMemorySubstrate`. Tests every `EntityClient` method in isolation: insert, resolve, checkout, commit, double-checkout error, undo_checkout, remove, persist with pending checkouts, undo_commit, unload.

### `derive_entity.rs`
Tests `#[derive(pari_macros::Entity)]` macro output on a minimal `TestRole` struct:
- `From<plain>` conversion, dirty flags, `merge_dirty_into`, `reset_dirty`
- `entity_ref()` accessor, async field accessors, async setters
- `#[should_panic(expected = "field not loaded")]` for uninitialized field access

### `entity_definitions.rs`
Smoke tests for all 9 entity structs: `EntityKind` values, parent types, `Tracked*` roundtrips.

### `error_compose.rs`
Tests `#[derive(ErrorCompose)]` macro: `fix_domain()`, `recoverability()`, `severity()`, `BatchError` aggregation, worst-case classification.

### `error_hierarchy.rs`
Tests error hierarchy: `PariError` wrapping, downcasting via `as_error::<T>()`.

### `tracked_serde.rs`
Serialization roundtrips for `Tracked<T>` and `EntityRef<T, P>` (both `NoParent` and `WorkflowParent`).

### `validate_entities.rs`
Runs `run_validations()` against all entity types with valid and invalid fixture data.

### `repo_substrate.rs`
Tests `RepoSubstrate` directly: file creation, overwrite, deletion, stale `.part/` cleanup on init.

### `storage_integration.rs`
Integration tests using the legacy `storage::RepoSubstrate`. Tests atomic persist with real filesystem.

### `substrate_pipeline.rs`
Tests pipeline components (`RepoCodec`, `RepoExecutor`, `RepoLocationResolver`) in isolation.

### `schema_coherence.rs`
Validates generated JSON schemas in `schemas/` against live entity instances using `jsonschema`.

---

## Test Patterns

```rust
// Isolated store session with real filesystem:
let dir = TempDir::new().unwrap();
EntityServer::with_test(RepoSubstrate::new(dir.path().to_path_buf()).unwrap(), || async {
    EntityClient::insert(...).await.unwrap();
    EntityClient::persist().await.unwrap();
}).await;

// In-memory only:
EntityServer::with_test(InMemorySubstrate::new(), || async { ... }).await;

// Pre-populated in-memory:
let s = InMemorySubstrate::new();
s.seed(role_any_ref("pm"), StoreEntity::Role(TrackedRole::from(make_role("pm"))));
EntityServer::with_test(s, || async { ... }).await;
```

**Mutation pattern** (checked-out entity):
```rust
let mut entity = EntityClient::checkout(role_ref("id")).await.unwrap();
if let StoreEntity::Role(ref mut r) = entity {
    r.set_name("New Name".to_string()).await.unwrap();
    // OR direct Arc replacement: r.name = Arc::new(TrackedField::with_value("New Name".to_string()));
}
entity.commit().await.unwrap();
```
