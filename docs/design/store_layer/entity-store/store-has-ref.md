# store-has-ref

**Store Layer → `store_layer/entity-store/`**

---

## Purpose

`has_ref` checks whether an entity exists — in the store or on the substrate. Used by validators to confirm cross-entity refs are valid. Calls `resolve` internally so a confirmed entity is always left in the store as a stub, avoiding a redundant substrate call on the next access.

---

## Signature

```rust
async fn has_ref(&mut self, any_ref: AnyEntityRef) -> Result<bool, SubstrateError>
```

Internal to `EntityServer`.

---

## Behavior

Delegates to `resolve(any_ref)` and maps the result:

| resolve result              | has_ref result         |
|-----------------------------|------------------------|
| `Ok(entity)`                | `Ok(true)`             |
| `Err(NotFound)`             | `Ok(false)`            |
| `Err(SubstrateError(e))`    | `Err(e)`               |

A successful `has_ref` leaves a stub in `entities` if one was not already present — subsequent resolves for the same ref are served from the store without a substrate call.
