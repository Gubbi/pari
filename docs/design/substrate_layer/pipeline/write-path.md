# write-path

**Owning layer: `substrate`**

---

## Purpose

The `persist` default implementation on the `Substrate` trait forms the write pipeline. It is called by `EntityServer` as Phase 2 of persist — after the pre-check and before the dirty reset. It consumes the store's lazy `EntityChange` iterator, maps each change through AssetMapper → `self.resolver()` → `self.codec()`, accumulates `AssetRequest`s, then executes them atomically via `self.executor()`.

---

## Execution Context

```
EntityServer::persist():

  Phase 1 — pre-check
    if checked_out non-empty → Err(PendingCheckouts)

  Phase 2 — execute  (this doc)
    substrate.persist(store.changes())

  Phase 3 — reset
    reset dirty flags on modified entities
    clear added, modified, removed
```

The store exposes pending changes as a lazy `EntityChange` iterator via `Store::changes()`. The substrate consumes this iterator at its own pace — no materialisation of the full change set upfront. On failure, the change lists and dirty flags are preserved so the caller can retry.

---

## Pipeline

```
persist:  EntityChange iterator → AssetMapper → self.resolver() → self.codec() → self.executor()
```

---

## Default Implementation

```rust
async fn persist(
    &self,
    changes: impl Iterator<Item = EntityChange<'_>>,
) -> Result<(), Vec<SubstrateError>> {
    let mut ops = Vec::new();

    for change in changes {
        let (entity, dirty_fields, is_removed) = match &change {
            EntityChange::Added(e)           => (e, None, false),
            EntityChange::Modified(e, dirty) => (e, Some(dirty), false),
            EntityChange::Removed(any_ref)   => {
                let schema = Self::schema_for(any_ref.kind());
                let stub_json = any_ref_to_stub_json(any_ref);
                let location = self.resolver().resolve(schema.ref_asset.path_template, &stub_json);
                ops.push(AssetRequest { location, op: AssetOp::Delete });
                continue;
            }
        };

        let schema = Self::schema_for(entity.kind());
        let entity_json = serde_json::to_value(entity)?;

        for asset in AssetMapper::select_for_write(schema, dirty_fields) {
            let location = self.resolver().resolve(asset.path_template, &entity_json);
            let field_values = extract_fields(&entity_json, asset.fields);
            let encoded = self.codec().encode(&field_values, asset.fields)?;
            let op = select_write_op(&change, asset.kind);
            ops.push(AssetRequest { location, op });
        }
    }

    self.executor().execute(ops).map(|_| ())
}
```

---

## Asset Selection for Writes

- **`Added`** — all assets are written; all fields are new.
- **`Modified`** — only assets containing at least one dirty field are written; other assets are skipped.
- **`Removed`** — only the `ref_asset` is deleted. Additional assets (e.g., a template file) are co-located and removed as a side effect of the LCA directory swap on RepoSubstrate. For substrates where assets are independent, the schema would list additional assets to delete explicitly.

```
Entity: Task, Modified, dirty_fields: ["name"]
  → "name" is in ref_asset (README.md)
  → write ref_asset only
  → template asset skipped (template_content not dirty)
```

For RepoSubstrate, writing an asset means writing all its fields — even fields that aren't dirty — because it's a full-file rewrite. The `ensure_mutable` constraint (see [59](../../workspace_layer/load/ensure-mutable.md)) guarantees all fields in the asset are loaded before any mutation.

---

## Op Selection

`select_write_op` maps entity state and `AssetKind` to the correct `AssetOp`:

- `distinguishes_create: true` → `Added` uses `Post`, `Modified` uses `Put`
- `distinguishes_create: false` → both use `Put`
- `supports_partial: true` + `Modified` → can use `Patch` with only dirty fields

RepoSubstrate has `distinguishes_create: false` and `supports_partial: false` — always `Put`.
