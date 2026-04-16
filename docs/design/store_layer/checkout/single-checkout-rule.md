# single-checkout-rule

**Owning layer: `store`**

---

## Purpose

At most one exclusive checkout may exist for a given entity at any time. `EntityServer` enforces this via `checked_out: HashSet<AnyEntityRef>`. Persist also fails if any checkouts are outstanding.

---

## checked_out

```rust
store.checked_out: HashSet<AnyEntityRef>
```

- Populated by `EntityClient::checkout()` on success
- Cleared by `entity.commit()` or `entity.undo_checkout()`

---

## Checkout Enforcement

Second checkout of the same entity returns `Err(AlreadyCheckedOut)` immediately — no blocking, no queueing. Concurrent checkouts of different entities are permitted.

---

## Persist Enforcement

```rust
EntityClient::persist() -> Err(PersistError::PendingCheckouts)  // if checked_out is non-empty
```

Persist fails fast if any entities are still checked out. The caller must commit or undo_checkout all checked-out entities before persisting.
