# persist-dirty-reset

**Owning layer: `store`**

---

## Purpose

After a successful substrate write, `EntityServer`'s persist handler resets all dirty state so the next persist cycle starts clean. Reset is scoped to entities in the change lists — not a full store traversal.

---

## Steps

1. **Reset dirty flags on modified entities** — for each entity in `modified`, call `entity.reset_dirty()`. This replaces each `Arc<TrackedField<T>>` with a new Arc where `dirty = false` and the `OnceLock` is preserved. Entities in `added` have `dirty = false` already — no reset needed.

2. **Clear the three lists** — empty `added`, `modified`, and `removed`.

No store_version counter is incremented — versions are not used in this design.

---

## On Failure

If the substrate write fails, none of the above steps run. The change lists and dirty flags are preserved so the caller can retry.

---

## Scope

Only entities in `modified` are touched for dirty reset. Entities that were loaded but not mutated are unaffected — their `TrackedField.dirty` flags are already `false`.
