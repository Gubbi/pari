# store-commit

**Owning layer: `store`**

---

## Purpose

`commit()` and `undo_checkout()` are methods on the tracked entity itself. They use `EntityServer::sender()` directly via the `request`/`send` internal helpers — no store reference needed by the caller.

---

## commit

```rust
async fn commit(&mut self) -> Result<(), CommitError>
```

Uses `request(StoreRequest::Commit { entity: self, any_ref })`. Inside EntityServer:

1. Run validations on the committed entity — return `Err(ValidationFailed)` if invalid
2. Call `entity.merge_dirty_into(store_entity)` — patch dirty fields onto the store's canonical entity; clean fields untouched
3. Update change lists:
   - Entity cannot be in `removed` — checkout would have returned `EntityNotFound`
   - If `has_dirty_fields()` and entity is in `added` → merge, then clear all dirty flags on the store entity (entity remains in `added`; dirty fields are irrelevant for full-write entities — the whole entity is always written on persist)
   - If `has_dirty_fields()` and entity is not in `added` → merge, then add to `modified`
   - If no dirty fields → no list update (no-op commit)
4. Remove `any_ref` from `checked_out`
5. Return `Ok(())`

---

## undo_checkout

```rust
async fn undo_checkout(&mut self) -> Result<(), UndoError>
```

Uses `send(StoreCommand::UndoCheckout { any_ref })`. Inside EntityServer: remove `any_ref` from `checked_out`. No merge, no validation. Changes are dropped with the entity.

---

## CommitError

```rust
enum CommitError {
    ValidationFailed {
        error_count: usize,
        errors: ValidationErrors,
        hint: Option<String>,
    },
    CrossReferenceCheckFailed(SubstrateError),
    StoreUnavailable(StoreError),
}
```
