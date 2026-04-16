# tracked-field

**Entity Layer → `entity_layer/field-primitives/`**

---

## Purpose

`TrackedField<T>` holds a single loaded value and tracks whether it has been mutated since last persist. It replaces `ScalarField` — `VecField` and `MapField` are retired.

---

## Structure

```rust
struct TrackedField<T> {
    value: OnceLock<T>,   // uninitialized = not loaded; initialized = loaded
    dirty: AtomicBool,    // true = mutated since last persist
}
```

## Why `AtomicBool` for `dirty`

`dirty` is set only at construction time by setters via `TrackedField::mutated(v)`. It is never set during loading — loading only initializes the `OnceLock`. However, `reset_dirty()` must clear `dirty` after a successful persist on fields inside `Arc<TrackedField<T>>` without replacing the Arc (store and client share the same Arc allocations). `AtomicBool` allows this in-place reset without requiring `&mut TrackedField`.

---

## Constructors

```rust
TrackedField::new()         // uninitialized, dirty = false  (stub fields)
TrackedField::loaded(v: T)  // OnceLock initialized, dirty = false  (substrate-loaded fields)
TrackedField::mutated(v: T) // OnceLock initialized, dirty = true   (setter-assigned fields)
```

---

`M: Optionality` is dropped. Optional domain fields encode optionality in T:

- `TrackedField<String>` — required field
- `TrackedField<Option<String>>` — optional field; OnceLock initialized with `None` = loaded, confirmed absent

---

## State Matrix

| `value` | `dirty` | Meaning |
|---|---|---|
| uninitialized | `false` | Not loaded |
| initialized | `false` | Loaded, clean |
| initialized | `true` | Loaded, dirty |
| uninitialized | `true` | Invalid |

---

## COW Wrapping

Each field on a tracked entity is `Arc<TrackedField<T>>`. Cloning a tracked entity clones the Arcs cheaply. Mutation replaces the Arc with a new one — see [04 · cow-field-convention](cow-field-convention.md).

---

## Loading

Loading is handled by `EntityServer`. When an accessor finds its `OnceLock` uninitialized, it sends `StoreRequest::Load` via `EntityClient`. `EntityServer` calls `OnceLock::set()` on its stored Arcs directly — since the accessor's `Arc<TrackedField<T>>` is the same allocation, the initialized value is immediately visible to the accessor after the request returns. No value travels back through the channel.
