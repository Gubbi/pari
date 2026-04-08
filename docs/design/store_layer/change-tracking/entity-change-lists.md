# entity-change-lists

**Store Layer → `store_layer/change-tracking/`**

---

## Purpose

The store maintains three `HashSet<AnyEntityRef>` lists to track which entities need action at persist time. These replace any version-scan approach — no `store_version` counter exists.

---

## Lists

```rust
store.added:    HashSet<AnyEntityRef>  // inserted since last persist
store.modified: HashSet<AnyEntityRef>  // mutated since last persist (existing entities)
store.removed:  HashSet<AnyEntityRef>  // removed since last persist
```

New entities are added via `EntityClient::insert()`. Modified entities are added at `EntityClient::commit()`. Removed entities are added at `EntityClient::remove()`. See [45 · store-structure](../entity-store/store-structure.md) for insert/remove paths.

---

## Transitions

### New-then-remove (before persist)

Entity inserted → in `added`. Then removed before persist:
- Remove from `added`
- Do NOT add to `removed` — entity was never persisted; substrate has nothing to delete
- Evict from store entries
- Net: no-op for persist ✓

### Remove-then-new (same key, before persist)

Entity removed → in `removed`, evicted from entries. Same key re-inserted:
- Remove from `removed`
- Add to `modified` — substrate still has the old version; this is an update, not a create
- Net: Modified op at persist ✓

---

## At Persist

The store exposes its three lists as a lazy `EntityChange` iterator via `Store::changes()`. The substrate's `persist` default implementation consumes this iterator, processing each change into `AssetRequest`s without materialising the full change set upfront.

Persist fails if `store.checked_out` is non-empty — see [44 · single-checkout-rule](../checkout/single-checkout-rule.md).

After successful persist, all three lists are cleared — see [64 · persist-dirty-reset](persist-dirty-reset.md). The iterator is specified in [61 · entity-change-iterator](entity-change-iterator.md).
