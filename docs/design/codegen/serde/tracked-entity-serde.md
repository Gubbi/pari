# tracked-entity-serde

**Codegen → `codegen/serde/`**

---

## Purpose

Tracked entities hold fields in `Arc<TrackedField<T>>` backed by `OnceLock`. Standard `#[derive(Serialize, Deserialize)]` does not know how to handle uninitialized fields. `#[derive(Entity)]` generates custom `Serialize` and `Deserialize` impls that are OnceLock-aware.

---

## Serialize

An initialized field is serialized normally. An uninitialized field is skipped (equivalent to `#[serde(skip_serializing_if = "Option::is_none")]` on an `Option`, but for `OnceLock`):

```rust
impl Serialize for TrackedRole {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let mut state = s.serialize_struct("Role", /* field count */)?;
        if let Some(v) = self.name.get() {
            state.serialize_field("name", v)?;
        }
        if let Some(v) = self.purpose.get() {
            state.serialize_field("purpose", v)?;
        }
        // ... repeat for all fields
        state.end()
    }
}
```

Only initialized fields appear in the output. This means a partially-loaded entity serializes to a partial document — callers that need a complete snapshot must ensure all fields are loaded before serializing.

---

## Deserialize

Each present key initializes the corresponding `OnceLock`. Absent keys are left uninitialized (not set to a default):

```rust
impl<'de> Deserialize<'de> for TrackedRole {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        // visitor reads map entries
        let mut name:    Option<String> = None;
        let mut purpose: Option<String> = None;
        // ... collect all keys ...

        let entity = TrackedRole {
            name:    Arc::new(TrackedField::new()),
            purpose: Arc::new(TrackedField::new()),
            // ...
        };
        if let Some(v) = name    { entity.name.initialize(v); }
        if let Some(v) = purpose { entity.purpose.initialize(v); }
        Ok(entity)
    }
}
```

`initialize()` is the OnceLock write used exclusively by the deserializer and the load path — it differs from the setter path (`set_name`) in that it does not run validators or mark fields dirty.

---

## Write-Once Merge Semantics

`OnceLock` permits exactly one write. The load path calls `initialize()` only when the field is not yet set:

```rust
fn initialize(&self, value: T) {
    let _ = self.inner.set(value); // no-op if already initialized
}
```

This means a partial load followed by a second partial load of the same field is safe — the first write wins. Setters (`set_name`) replace the value via a separate mechanism that bypasses the `OnceLock` guard; they are the only permitted second-write path and they always mark the field dirty.

---

## Codec Integration

### Read path

The substrate codec reads a file (or record) into a `serde_json::Value` (or equivalent), then calls `TrackedRole::deserialize`. Only the keys present in the source document are initialized. Missing optional fields remain uninitialized and will trigger a substrate load if accessed later.

### Write path

Before persisting, the store calls `TrackedRole::serialize` to produce the output document. Because uninitialized fields are skipped, only fields that were either loaded or explicitly set appear in the output. The substrate codec writes this document atomically.

---

## Summary

| Concern | Behavior |
|---|---|
| Initialized field — serialize | included in output |
| Uninitialized field — serialize | skipped |
| Present key — deserialize | `OnceLock` initialized |
| Absent key — deserialize | `OnceLock` left empty |
| Concurrent first-load race | first writer wins; subsequent are no-ops |
| Setter vs deserializer write | setter marks dirty; deserializer does not |
