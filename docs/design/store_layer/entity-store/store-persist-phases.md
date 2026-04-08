# store-persist-phases

**Store Layer → `store_layer/entity-store/`**

---

## Purpose

`persist` on `EntityServer` runs in three phases: pre-check, execute, and reset. It is triggered via `EntityClient::persist()`.

---

## Phase 1 — Pre-check

Fail immediately if `checked_out` is non-empty:

```rust
if !self.checked_out.is_empty() {
    return Err(PersistError::PendingCheckouts);
}
```

No substrate call is made until all checkouts are resolved.

---

## Phase 2 — Execute

Call `substrate.persist(self.changes())`, passing a lazy `EntityChange` iterator over the three change lists — `added`, `modified`, `removed`. The substrate's default implementation maps each change to `AssetRequest`s and executes them atomically.

The iterator and `EntityChange` enum are specified in [61 · entity-change-iterator](../change-tracking/entity-change-iterator.md).

---

## Phase 3 — Reset

On success only — failure preserves all state for retry:

- Reset dirty flags on each entity in `modified` (entities in `added` have `dirty = false` already)
- Clear `added`, `modified`, and `removed`

See [64 · persist-dirty-reset](../change-tracking/persist-dirty-reset.md) for reset details.

---

## PersistError

```rust
enum PersistError {
    PendingCheckouts,
    SubstrateError(Vec<SubstrateError>),
}
```
