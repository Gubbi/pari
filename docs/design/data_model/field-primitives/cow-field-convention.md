# cow-field-convention

**Data Model → `data_model/field-primitives/`**

---

## Purpose

Entity fields on tracked entities are wrapped in `Arc<TrackedField<T>>`. This makes checkout (COW clone) uniformly cheap across all field types — `String`, `Raci`, `Artifact`, or any other `T` — without requiring a special string convention.

---

## The Wrapper

```rust
// Fields on a tracked entity
name:    Arc<TrackedField<String>>,
purpose: Arc<TrackedField<String>>,
raci:    Arc<TrackedField<Raci>>,
```

The `Arc` wraps the entire `TrackedField<T>`, not just the inner value. The refcount tracks ownership of the field as a whole unit.

---

## Checkout

When `EntityClient::checkout()` clones the entity, each `Arc<TrackedField<T>>` clone is an atomic refcount increment — no field data is copied regardless of `T`'s size or complexity.

```rust
let mut clone = role.clone();
// clone.name and role.name point to the same Arc<TrackedField<String>>
// clone.raci and role.raci point to the same Arc<TrackedField<Raci>>
// All fields shared — O(N fields) refcount bumps, zero heap allocations
```

After checkout, store and clone share all field Arcs. The store copy is unaffected by setters called on the clone.

---

## Setter

A setter is async. It first sends `StoreRequest::EnsureMutable` to `EntityServer` (which loads any prerequisites and the field itself if required), then replaces the `Arc` entirely using `TrackedField::mutated(v)`:

```rust
pub async fn set_name(&mut self, value: String) -> Result<(), SetterError> {
    EntityClient::ensure_mutable(self.entity_ref.to_any(), "name").await?;
    self.name = Arc::new(TrackedField::mutated(value));
    Ok(())
}
```

The store's `Arc<TrackedField<String>>` for `name` is unaffected — it still points to the original. The clone now holds a new Arc. One heap allocation per mutation.

---

## `merge_dirty_into`

At commit, dirty fields on the clone are merged back into the store entity by overwriting the store's Arc with the clone's Arc:

```rust
fn merge_dirty_into(&self, store_entity: &mut TrackedRole) {
    if self.name.is_dirty() {
        store_entity.name = Arc::clone(&self.name);
    }
    if self.raci.is_dirty() {
        store_entity.raci = Arc::clone(&self.raci);
    }
    // ...
}
```

No allocation — just refcount bumps. After merge, the store holds the clone's `Arc<TrackedField<T>>` for each merged field, with `dirty: true` intact.

---

## Dirty Reset

Dirty flags are not cleared at merge time. Reset happens as a separate lifecycle event after a successful persist. See [64 · persist-dirty-reset](../../store_layer/change-tracking/persist-dirty-reset.md).
