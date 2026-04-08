# codec

**Substrate Layer → `substrate_layer/pipeline/`**

---

## Purpose

`Codec` translates between substrate-encoded bytes and a field-keyed JSON map. It is entirely schema-driven and entity-agnostic — all per-entity variation lives in the `FieldMapping` slice passed to each call.

---

## Trait

```rust
trait Codec {
    type Slot: Slot;
    type Encoded;

    fn encode(
        &self,
        fields: &HashMap<&str, serde_json::Value>,
        schema: &[FieldMapping<Self::Slot>],
    ) -> Result<Self::Encoded, CodecError>;

    fn decode(
        &self,
        raw: &Self::Encoded,
        schema: &[FieldMapping<Self::Slot>],
    ) -> Result<HashMap<String, serde_json::Value>, CodecError>;
}
```

- `encode` — takes a map of field name → JSON value and a slot mapping; produces the substrate's encoded format
- `decode` — takes encoded bytes and a slot mapping; produces a map of field name → JSON value

The codec does not know which entity type it is processing. It sees only field names and slot values. One codec implementation serves all entity types for a given substrate.

---

## Encoded Intermediary

`serde_json::Value` is the intermediary between the `Store` and the codec. It never crosses the Store Layer–substrate boundary directly:

```
Read path (inside substrate):
  Executor → raw bytes (Encoded)
  Codec.decode() → HashMap<String, serde_json::Value>
  serde_json::from_value() → TrackedEntity (partial, only decoded fields populated)
  Return to Store

Write path (inside substrate):
  serde_json::to_value(&tracked_entity) → serde_json::Value (object)
  Extract dirty field values → HashMap<&str, Value>
  Codec.encode() → Encoded
  Executor writes
```

---

## Serde on TrackedEntity

The tracked entity's serde impl (`codegen/serde/tracked-entity-serde`, not yet written) is OnceLock-aware:
- **Serialize**: OnceLock initialized → serialize field; uninitialized → skip field entirely
- **Deserialize**: field present in JSON → initialize OnceLock with parsed value; field absent → leave OnceLock uninitialized

This means a partial decode (only some fields in the JSON) correctly produces a partial `TrackedEntity`.

---

## CodecError

```rust
struct CodecError {
    field: String,
    message: String,
}
```
