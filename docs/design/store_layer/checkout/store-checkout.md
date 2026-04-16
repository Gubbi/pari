# store-checkout

**Owning layer: `store`**

---

## Purpose

`EntityClient::checkout()` grants exclusive mutable access to a single entity. It returns an owned `TrackedEntity` clone. The server marks the entity as checked out — no second checkout is permitted until the caller calls `commit()` or `undo_checkout()` on the entity.

---

## Signature

```rust
EntityClient::checkout(any_ref: AnyEntityRef) -> Result<TrackedEntity, CheckoutError>
```

---

## Steps (inside EntityServer)

1. Check `checked_out` — if `any_ref` is present, return `Err(AlreadyCheckedOut { entity_ref, hint: None })`
2. Look up `entities` — if `any_ref` is absent, return `Err(EntityNotFound { entity_ref, hint: None })`
   Checkout does NOT hit the substrate — it requires the entity to already be in the store.
   Use `resolve` first if the entity may not be in the store.
3. Clone the entity into an owned `TrackedEntity`
4. Insert `any_ref` into `checked_out`
5. Return the cloned entity

---

## CheckoutError

```rust
enum CheckoutError {
    AlreadyCheckedOut {
        entity_ref: String,
        hint: Option<String>,
    },
    EntityNotFound {
        entity_ref: String,
        hint: Option<String>,
    },
    StoreUnavailable(StoreError),
}
```

---

## Notes

- Checkout does not load fields — callers load what they need before or within the checkout via `ensure_mutable()`
- Concurrent checkouts of different entities are permitted
- The returned `TrackedEntity` carries `commit()` and `undo_checkout()` methods that use `EntityServer::sender()` directly — no store reference needed by the caller
- `undo_checkout()` is the undo of checkout: the `checked_out` lock is released and all mutations on the caller's copy are dropped. The store entity is unchanged.
- `undo_commit()` and `unload()` are on `EntityClient` — they operate on already-committed or clean entities, not on a checked-out copy
