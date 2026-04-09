# async-accessor-variants

**Codegen → `codegen/async/`**

---

## Purpose

`#[derive(Entity)]` generates async accessor and setter methods as inherent methods on each tracked entity struct. All methods are async — there are no sync counterparts and no `_async` suffix. This document covers the generated signatures.

---

## Accessor

Each field gets a single async method. The method checks the `OnceLock` and sends `StoreRequest::Load` if the field is not yet initialized:

```rust
pub async fn name(&self) -> Result<&str, LoadError> {
    if self.name.value.get().is_none() {
        // EntityServer will call OnceLock::set() on the shared Arc directly.
        // No value travels back — Arc sharing makes the write immediately visible.
        EntityClient::load(self.entity_ref.to_any(), "name").await?;
    }
    Ok(self.name.value.get().expect("field not loaded"))
}
```

`EntityClient::load` sends `StoreRequest::Load { any_ref, field }` to `EntityServer`. The server fetches the field from the substrate and calls `OnceLock::set()` on the store entity's existing `Arc<TrackedField<T>>`. Since the accessor's Arc is the same allocation, the value is immediately visible after the request returns.

The `OnceLock` write-once guarantee means concurrent callers racing on first load will each trigger the load, but only the first `OnceLock::set()` wins — subsequent sets are no-ops.

---

## Setter

Each field gets a single async setter. Setters send `StoreRequest::EnsureMutable` before mutating:

```rust
pub async fn set_name(
    &mut self,
    value: String,
) -> Result<(), SetterError> {
    EntityClient::ensure_mutable(self.entity_ref.to_any(), "name").await?;
    self.name = Arc::new(TrackedField::mutated(value));
    Ok(())
}
```

Steps:
1. Send `StoreRequest::EnsureMutable { any_ref, field }` — `EntityServer` loads prerequisites and (if required) the field itself before mutation is allowed. See [ensure-mutable](../../workspace_layer/load/ensure-mutable.md).
2. If `ensure_mutable` returns `Err`: return the error without modifying the field.
3. Replace the Arc with a new `TrackedField::mutated(value)` — `OnceLock` pre-initialized, `dirty = true`.

---

## Generated Output Summary

For each field `name: String` on `Role`:

| Method | On | Signature |
|---|---|---|
| `name` | `TrackedRole` | `async fn name(&self) -> Result<&str, LoadError>` |
| `set_name` | `TrackedRole` | `async fn set_name(&mut self, v: String) -> Result<(), SetterError>` |
