# tracked-field

**Data Model → `data_model/field-primitives/`**

---

## Purpose

`TrackedField<T>` holds a single loaded value and tracks whether it has been mutated since last persist. It replaces `ScalarField` — `VecField` and `MapField` are retired.

---

## Structure

```rust
struct TrackedField<T> {
    value: OnceLock<T>,  // uninitialized = not loaded; initialized = loaded
    dirty: bool,         // true = mutated since last persist
}
```

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

## Loading and Mutation

Loading and mutation are handled externally — by the tracked entity's accessors and `ensure_mutable()`. See tracked entity topics (30–35).
