# async-accessor-variants

**Codegen → `codegen/async/`**

---

## Purpose

`#[derive(Entity)]` generates async accessor and setter methods as inherent methods on each tracked entity struct. All methods are async — there are no sync counterparts and no `_async` suffix. This document covers the generated signatures.

---

## Accessor

Each field gets a single async method. The method performs the OnceLock check and issues a substrate load if the field is not yet initialized:

```rust
pub async fn name(&self) -> Result<&str, LoadError> {
    self.name.get_or_load().await
}
```

`get_or_load()` returns the cached value immediately if the field is already initialized. If not, it issues a substrate read, stores the result in the `OnceLock`, and returns a reference to it. The `OnceLock` write-once guarantee means concurrent callers racing on first load will each attempt the load, but only the first writer wins — subsequent loads are no-ops.

---

## Setter

Each field gets a single async setter. Setters validate before committing:

```rust
pub async fn set_name(
    &mut self,
    value: String,
) -> Result<(), SetError> {
    let violations = validate_name(self).await;
    if !violations.is_empty() {
        return Err(SetError::ValidationFailed(violations));
    }
    self.name.set(value);
    self.mark_dirty_name();
    Ok(())
}
```

Steps:
1. Run field-level validators against `self` — validator functions take `&TrackedRole` directly.
2. If violations: return `Err` without modifying the field.
3. Write to the `OnceLock` (replacing any previously initialized value — setters are the only permitted second write).
4. Mark the field dirty for change tracking.

---

## Validators

Validator functions are `TrackedRole`-specific async functions. There are no plain-entity validator counterparts — plain entities are valid by construction.

```rust
async fn validate_name(e: &TrackedRole) -> Vec<RuleViolation> {
    let name = e.name().await.unwrap();
    if name.is_empty() {
        vec![RuleViolation::new("name", "must not be empty")]
    } else {
        vec![]
    }
}
```

---

## Generated Output Summary

For each field `name: String` on `Role`:

| Method | On | Signature |
|---|---|---|
| `name` | `TrackedRole` | `async fn name(&self) -> Result<&str, LoadError>` |
| `set_name` | `TrackedRole` | `async fn set_name(&mut self, v: String) -> Result<(), SetError>` |
