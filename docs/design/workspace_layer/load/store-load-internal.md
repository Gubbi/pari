# store-load-internal

**Workspace Layer → `workspace_layer/load/`**

---

## Purpose

`EntityServer::load(any_ref, fields)` is the internal method that fetches fields from the substrate and merges them into the cached entity. Called by field accessors on first access — when an accessor finds its `OnceLock` uninitialized, it sends a `StoreRequest::Load` via `EntityServer::sender()`. There is no explicit load API on tracked entities.

---

## Algorithm

```
load(any_ref, fields):

  0. Cache-hit short-circuit:
       remove from fields any field where cached OnceLock is initialized
       if no fields remain → return (all already loaded)

  1. For each remaining field:
       strategy = S::load_strategy(any_ref.kind(), field)
       if strategy.prerequisites not loaded:
         recurse: load(any_ref, strategy.prerequisites)

  2. Fetch from substrate:
       substrate.load(entity, fields)   // fields=&[] means all

  3. Expand cross-entity refs in the result:
       call all_refs() on the returned TrackedEntity
       filter to refs not already in store
       batch: substrate.exists(refs_not_in_store)
       for each confirmed-existing ref: insert stub into store
       for each errored ref: skip — do not insert, do not fail the load
       // This prefetch step is only an optimization ahead of validation.
       // Validation still decides whether the loaded data is acceptable.

  4. Enrich and validate:
       a. Merge store's already-initialized fields INTO the loaded result (for validation context)
          — fields already loaded in the store are copied into the result before validation
          — this gives validators a complete picture even when only a subset of fields was fetched
       b. Run validations on the enriched result
          — structural, entity-local semantic, and cross-entity semantic rules
          — unloaded fields are skipped
          Err(ValidationFailed) if any rule fails

  5. Initialize store's existing Arcs in-place:
       For each field newly loaded in the result:
         call OnceLock::set() on the store entity's existing Arc<TrackedField<T>>
         (write-once: already-initialized OnceLocks are not overwritten)
       The store's TrackedEntity is NEVER replaced — only its OnceLocks are initialized.
       Arc sharing propagates these writes to all holders (client clones) immediately.
```

---

## Write-Once Merge Semantics

The merge in step 5 is write-once: a field already loaded (OnceLock initialized) is never overwritten by a subsequent load result.

The rule fires in multi-round progressive loading. Example: `Task.template_content` depends on `Task.artifact`. Loading `template_content` triggers two substrate calls — first for the ref_asset (which returns `artifact` and all other ref_asset fields), then for the asset (which may return ref_asset fields again alongside `template_content`). Write-once prevents the second call from clobbering `artifact` that was already loaded in the first round.

Within a single round the rule is a no-op — each field is only populated once.

---

## Prerequisite Resolution

Prerequisites form a DAG. Resolution is recursive and depth-first: a prerequisite is fully loaded before the dependent field is fetched. Cycles are impossible — `RefAssetDef` has no `path_deps`, and `AssetDef.path_deps` only references fields in the ref_asset.

---

## Ref Expansion

After each load round, `all_refs()` surfaces cross-entity refs that were just populated. These are pre-fetched as stubs via a single batched `substrate.exists()` call. This is an optimization — the same refs would be resolved individually during validation's `has_ref()` checks. Batch pre-fetch avoids N serial substrate round-trips.

Only refs not already in the store are included in the batch. Stubs are inserted without being marked in `added` — they are not user-created entities.

Errors from individual `exists()` checks are non-fatal and do not abort the prefetch step. Confirmed refs are inserted; errored refs are skipped. The store retains only validated facts — a stub in the store means existence was confirmed. Prefetch is only a best-effort optimization before validation begins; if validation cannot validate a required ref, the load fails and the fetched data is not merged.

---

## Where Called

`EntityServer::load` is called by field accessors on first access. No `&mut self` is needed — the EntityServer owns the canonical cached entity; the accessor sends a channel request, the server merges into its copy, and the accessor initializes its `OnceLock` from the result.

```rust
// Transparently loads on first access via OnceLock
pub async fn name(&self) -> Result<&str, LoadError> {
    if self.name.value.get().is_none() {
        // EntityServer will call OnceLock::set() on the shared Arc directly.
        // No value travels back — Arc sharing makes the write immediately visible.
        EntityClient::load(self.entity_ref.to_any(), "name").await?;
    }
    Ok(self.name.value.get().expect("field not loaded"))
}
```

See [async-accessor-variants](../../codegen/async/async-accessor-variants.md).
