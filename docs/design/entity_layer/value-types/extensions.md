# extensions

**Owning layer: `entity`**

---

## Purpose

Every entity carries an `extensions` field for user-defined metadata. Keys must be prefixed with `x-` — a convention that reserves the un-prefixed namespace for first-class fields.

---

## Type Definition

```rust
pub type Extensions = HashMap<String, serde_json::Value>;
```

A type alias — no newtype wrapper. Values are untyped `serde_json::Value`; the schema enforces the `x-` prefix rule, not the type system.

---

## Usage on Entity Structs

```rust
#[derive(Serialize, Deserialize)]
pub struct Role {
    // ... other fields ...
    #[serde(flatten)]
    pub extensions: Extensions,
}
```

`#[serde(flatten)]` merges the map's keys into the surrounding JSON object on both serialize and deserialize — extension keys appear at the top level of the entity, not nested under an `"extensions"` key.

---

## The `x-` Prefix Rule

Only keys beginning with `x-` are valid (e.g. `x-owner`, `x-priority`). This rule is enforced by `validation`, not by serde or the type system — any string key can be inserted at runtime, but validation rejects non-`x-` keys. See `validate_extensions()` in the `validation` layer.

---

## JSON Schema — Post-Processing

`schemars` 0.8 generates a `patternProperties` entry for flattened `HashMap` fields but does not automatically add `additionalProperties: false`. Without it, non-`x-` keys would be permitted by the schema.

A post-processing step in `cargo xtask` adds `"additionalProperties": false` to any schema object that has `patternProperties`, closing the gap:

```json
{
  "patternProperties": {
    "^x-": {}
  },
  "additionalProperties": false
}
```
