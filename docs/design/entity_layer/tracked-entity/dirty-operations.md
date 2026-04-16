# dirty-operations

**Entity Layer → `entity_layer/tracked-entity/`**

---

## Purpose

Dirty operations on a tracked entity inspect and transfer field-level mutation state. They are used at commit time (merge dirty fields into the store's canonical entity) and at persist time (reset dirty flags after successful write).

---

## has_dirty_fields

```rust
fn has_dirty_fields(&self) -> bool
```

Returns `true` if any `Arc<TrackedField<T>>` on the entity has `dirty = true`. Used at commit to decide whether to add the entity to the store's `modified` list.

---

## merge_dirty_into

```rust
fn merge_dirty_into(&self, target: &mut Self)
```

Called at commit. For each field where `self.field.dirty == true`, replaces the corresponding field Arc on `target` with a clone of `self`'s Arc. Fields where `dirty == false` are left untouched on `target` — they may have been loaded independently.

This is a field-level patch, not a full replacement.

---

## reset_dirty

```rust
fn reset_dirty(&mut self)
```

Called after a successful persist. For each `Arc<TrackedField<T>>`, replaces the Arc with a new one where `dirty = false` and the `OnceLock` is preserved. Only called on entities in the `modified` list — not a full store traversal.
