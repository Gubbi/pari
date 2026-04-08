# store-checkout

**Store Layer → `store_layer/checkout/`**

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

1. Check `checked_out` — if `any_ref` is present, return `Err(AlreadyCheckedOut)`
2. Resolve the entity (must already be in the store — checkout does not trigger a load)
3. Clone the entity into an owned `TrackedEntity`
4. Insert `any_ref` into `checked_out`
5. Return the cloned entity

---

## CheckoutError

```rust
enum CheckoutError {
    AlreadyCheckedOut,
    EntityNotFound,
}
```

---

## Notes

- Checkout does not load fields — callers load what they need before or within the checkout via `ensure_mutable()`
- Concurrent checkouts of different entities are permitted
- The returned `TrackedEntity` carries `commit()` and `undo_checkout()` methods that use `EntityServer::sender()` directly — no store reference needed by the caller
- `undo_checkout()` is the undo of checkout: the `checked_out` lock is released and all mutations on the caller's copy are dropped. The store entity is unchanged.
- `undo_commit()` and `unload()` are on `EntityClient` — they operate on already-committed or clean entities, not on a checked-out copy
