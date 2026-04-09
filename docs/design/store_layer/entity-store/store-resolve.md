# store-resolve

**Store Layer → `store_layer/entity-store/`**

---

## Purpose

`resolve` returns an owned tracked entity for a given `AnyEntityRef`. On a cache hit it returns a cheap clone. On a cache miss it hits the substrate to confirm the entity exists, creates a stub, inserts it into the store, and returns the stub. The store always contains validated, substrate-confirmed data — stubs are only created after a successful `substrate.exists()` check.

---

## Signature

```rust
async fn resolve(&mut self, any_ref: AnyEntityRef) -> Result<TrackedEntity, ResolveError>
```

Internal to `EntityServer`. Called from the actor loop when handling `StoreRequest::Resolve`.

---

## Steps

1. If `entities` contains `any_ref` → return clone of the stored entity
2. Call `substrate.exists(&[any_ref])` — async batch existence check; returns `Result<Vec<bool>, SubstrateError>`
3. If substrate error → return `Err(ResolveError::Substrate(e))`
4. If result[0] is false → return `Err(ResolveError::NotFound { entity_ref: any_ref.id(), hint: None })`
5. Construct a stub: tracked entity with `entity_ref` populated, all fields' OnceLock uninitialized, `dirty = false`
6. Insert stub into `entities`
7. Return clone of stub

---

## ResolveError

```rust
enum ResolveError {
    NotFound {
        entity_ref: String,
        hint: Option<String>,
    },
    Substrate(SubstrateError),
    StoreUnavailable(StoreError),
}
```

---

## Stub State

The returned stub is in the `Stub` load state — `entity_ref` only, all fields' OnceLock uninitialized. Field data is loaded transparently on first access via field accessors. The stub in the store acts as a sentinel: a subsequent resolve for the same ref returns the stub directly without hitting the substrate again.
