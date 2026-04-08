# Task 08 — Tracked Entity Serde

## Scope

Implement OnceLock-aware `Serialize` and `Deserialize` for all tracked entity structs. Standard `#[derive(Serialize, Deserialize)]` does not handle uninitialized `Arc<TrackedField<T>>` fields correctly — initialized fields must be serialized normally; uninitialized fields must be skipped. This task generates those custom impls via `#[derive(Entity)]` (update to Task 03's macro).

---

## Files

- `pari-macros/src/lib.rs` — extend `#[derive(Entity)]` to emit Serialize/Deserialize for TrackedX
- `Cargo.toml` — ensure `serde` with `derive` feature is present

---

## Dependencies

- Task 01: `TrackedField::get()`, `TrackedField::initialize()`
- Task 03: `TrackedX` struct; `new_initialized`
- Task 05: All plain entity types and field types (must impl `Serialize`/`Deserialize`)

---

## Serialize Behavior

An initialized field is serialized normally. An uninitialized field is skipped entirely (not emitted as `null`):

```rust
impl Serialize for TrackedRole {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;
        // Count initialized fields for the struct header
        let field_count = [
            self.name.get().is_some(),
            self.description.get().is_some(),
            self.purpose.get().is_some(),
            self.traits.get().is_some(),
            self.extensions.get().is_some(),
        ].iter().filter(|&&b| b).count();

        let mut state = s.serialize_struct("Role", field_count)?;
        if let Some(v) = self.name.get()        { state.serialize_field("name", v)?; }
        if let Some(v) = self.description.get() { state.serialize_field("description", v)?; }
        if let Some(v) = self.purpose.get()     { state.serialize_field("purpose", v)?; }
        if let Some(v) = self.traits.get()      { state.serialize_field("traits", v)?; }
        if let Some(v) = self.extensions.get()  { state.serialize_field("extensions", v)?; }
        state.end()
    }
}
```

`entity_ref` is always present — serialize it unconditionally at the start of every tracked entity.

`extensions` uses `#[serde(flatten)]` on the plain entity. In `TrackedX::Serialize`, the extensions map must be serialized with its keys flattened into the surrounding object. Use `serde_json::Map` or a newtype to achieve this:

```rust
// Flatten extensions during serialization:
if let Some(ext) = self.extensions.get() {
    for (k, v) in ext {
        state.serialize_field(
            // CAUTION: field name is dynamic — use serde_json's map serializer
            // or an intermediate Value to handle flatten
        )?;
    }
}
```

**Note on flatten**: `SerializeStruct` does not natively support dynamic field names for flatten. Use an intermediate `serde_json::Value::Object` to collect all static fields, then merge extension keys in, and serialize the resulting map:

```rust
impl Serialize for TrackedRole {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        use serde_json::{Map, Value};
        let mut map = Map::new();
        if let Some(v) = self.name.get()        { map.insert("name".to_string(), serde_json::to_value(v).map_err(serde::ser::Error::custom)?); }
        if let Some(v) = self.description.get() { map.insert("description".to_string(), serde_json::to_value(v).map_err(serde::ser::Error::custom)?); }
        if let Some(v) = self.purpose.get()     { map.insert("purpose".to_string(), serde_json::to_value(v).map_err(serde::ser::Error::custom)?); }
        if let Some(v) = self.traits.get()      { map.insert("traits".to_string(), serde_json::to_value(v).map_err(serde::ser::Error::custom)?); }
        if let Some(v) = self.extensions.get()  { map.extend(v.iter().map(|(k, v)| (k.clone(), v.clone()))); }
        Value::Object(map).serialize(s)
    }
}
```

This handles flatten correctly at the cost of an intermediate allocation. Acceptable for this use case.

---

## Deserialize Behavior

Present keys initialize the corresponding `OnceLock`. Absent keys leave the field uninitialized (not defaulted):

```rust
impl<'de> Deserialize<'de> for TrackedRole {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        use serde::de::{MapAccess, Visitor};
        use std::fmt;

        struct TrackedRoleVisitor;
        impl<'de> Visitor<'de> for TrackedRoleVisitor {
            type Value = TrackedRole;
            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                write!(f, "a Role object")
            }

            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<TrackedRole, A::Error> {
                let mut entity_ref: Option<EntityRef<Role>> = None;
                let mut name:        Option<String> = None;
                let mut description: Option<Option<String>> = None;
                let mut purpose:     Option<String> = None;
                let mut traits:      Option<Option<Vec<String>>> = None;
                let mut extensions:  Option<Extensions> = None;

                while let Some(key) = map.next_key::<String>()? {
                    match key.as_str() {
                        "entity_ref"  => entity_ref  = Some(map.next_value()?),
                        "name"        => name        = Some(map.next_value()?),
                        "description" => description = Some(map.next_value()?),
                        "purpose"     => purpose     = Some(map.next_value()?),
                        "traits"      => traits      = Some(map.next_value()?),
                        k if k.starts_with("x-") => {
                            // Extension key — collect into extensions map
                            let v: serde_json::Value = map.next_value()?;
                            extensions.get_or_insert_with(HashMap::new).insert(key.clone(), v);
                        }
                        _ => { let _ = map.next_value::<serde_json::Value>()?; } // ignore unknown
                    }
                }

                let entity_ref = entity_ref.ok_or_else(|| serde::de::Error::missing_field("entity_ref"))?;

                let tracked = TrackedRole {
                    entity_ref,
                    name:        Arc::new(TrackedField::new()),
                    description: Arc::new(TrackedField::new()),
                    purpose:     Arc::new(TrackedField::new()),
                    traits:      Arc::new(TrackedField::new()),
                    extensions:  Arc::new(TrackedField::new()),
                };
                if let Some(v) = name        { tracked.name.initialize(v); }
                if let Some(v) = description { tracked.description.initialize(v); }
                if let Some(v) = purpose     { tracked.purpose.initialize(v); }
                if let Some(v) = traits      { tracked.traits.initialize(v); }
                if let Some(v) = extensions  { tracked.extensions.initialize(v); }
                Ok(tracked)
            }
        }

        d.deserialize_map(TrackedRoleVisitor)
    }
}
```

**`entity_ref` is required** — deserialization fails with a missing-field error if it is absent.

**Extension keys** (`x-` prefix): These are flattened in the source document. The visitor must collect any `x-` prefixed key into the extensions map, since `serde(flatten)` is not available in a manual visitor. The `_` arm ignores unknown non-x keys.

---

## TDD: Tests to Write First

```rust
// tests/tracked_serde.rs
use pari::entities::role::{Role, TrackedRole};
use pari::entity::EntityRef;
use pari::tracked::TrackedField;
use std::sync::Arc;
use std::collections::HashMap;

// Helper: build a fully-populated TrackedRole
fn full_tracked_role() -> TrackedRole {
    let plain = Role {
        entity_ref:  EntityRef::new("eng-lead"),
        name:        "Engineering Lead".to_string(),
        description: Some("Senior tech lead".to_string()),
        purpose:     "Leads engineering".to_string(),
        traits:      Some(vec!["reviewer".to_string()]),
        extensions:  {
            let mut m = HashMap::new();
            m.insert("x-owner".to_string(), serde_json::json!("alice"));
            m
        },
    };
    TrackedRole::from(plain)
}

// Helper: build a partially-populated TrackedRole (only name initialized)
fn partial_tracked_role() -> TrackedRole {
    TrackedRole {
        entity_ref:  EntityRef::new("eng-lead"),
        name:        Arc::new(TrackedField::new_initialized("Engineering Lead".to_string())),
        description: Arc::new(TrackedField::new()),  // uninitialized
        purpose:     Arc::new(TrackedField::new()),  // uninitialized
        traits:      Arc::new(TrackedField::new()),  // uninitialized
        extensions:  Arc::new(TrackedField::new()),  // uninitialized
    }
}

// --- Serialize ---

#[test]
fn full_tracked_role_serializes_all_fields() {
    let tracked = full_tracked_role();
    let json = serde_json::to_value(&tracked).unwrap();
    assert!(json.get("name").is_some());
    assert!(json.get("description").is_some());
    assert!(json.get("purpose").is_some());
    assert!(json.get("traits").is_some());
    assert!(json.get("x-owner").is_some()); // extensions are flattened
}

#[test]
fn partial_tracked_role_skips_uninitialized_fields() {
    let tracked = partial_tracked_role();
    let json = serde_json::to_value(&tracked).unwrap();
    assert!(json.get("name").is_some());
    assert!(json.get("description").is_none(), "uninitialized field must be absent");
    assert!(json.get("purpose").is_none());
}

#[test]
fn extensions_are_flattened_in_serialized_output() {
    let tracked = full_tracked_role();
    let json = serde_json::to_value(&tracked).unwrap();
    // Extension key appears at top level, not under "extensions"
    assert!(json.get("x-owner").is_some());
    assert!(json.get("extensions").is_none());
}

// --- Deserialize ---

#[test]
fn deserialize_full_json_initializes_all_fields() {
    let json = serde_json::json!({
        "entity_ref": { "id": "eng-lead", "kind": "Role" },
        "name": "Engineering Lead",
        "description": "Senior tech lead",
        "purpose": "Leads engineering",
        "traits": ["reviewer"],
        "x-owner": "alice"
    });
    let tracked: TrackedRole = serde_json::from_value(json).unwrap();
    assert_eq!(tracked.name.get(), Some(&"Engineering Lead".to_string()));
    assert_eq!(tracked.description.get(), Some(&Some("Senior tech lead".to_string())));
    assert_eq!(tracked.purpose.get(), Some(&"Leads engineering".to_string()));
    assert!(tracked.extensions.get().is_some());
    assert_eq!(
        tracked.extensions.get().unwrap().get("x-owner"),
        Some(&serde_json::json!("alice"))
    );
}

#[test]
fn deserialize_partial_json_leaves_missing_fields_uninitialized() {
    let json = serde_json::json!({
        "entity_ref": { "id": "eng-lead", "kind": "Role" },
        "name": "Engineering Lead"
    });
    let tracked: TrackedRole = serde_json::from_value(json).unwrap();
    assert_eq!(tracked.name.get(), Some(&"Engineering Lead".to_string()));
    assert!(tracked.purpose.get().is_none(), "absent field must remain uninitialized");
    assert!(tracked.description.get().is_none());
}

#[test]
fn deserialize_missing_entity_ref_returns_error() {
    let json = serde_json::json!({ "name": "Engineering Lead" });
    let result: Result<TrackedRole, _> = serde_json::from_value(json);
    assert!(result.is_err(), "entity_ref is required");
}

#[test]
fn deserialized_fields_are_not_dirty() {
    let json = serde_json::json!({
        "entity_ref": { "id": "eng-lead", "kind": "Role" },
        "name": "Engineering Lead",
        "purpose": "Leads"
    });
    let tracked: TrackedRole = serde_json::from_value(json).unwrap();
    // initialize() does not set dirty
    assert!(!tracked.name.is_dirty());
    assert!(!tracked.purpose.is_dirty());
}

// --- Roundtrip ---

#[test]
fn full_serialize_deserialize_roundtrip() {
    let original = full_tracked_role();
    let json = serde_json::to_value(&original).unwrap();
    let restored: TrackedRole = serde_json::from_value(json).unwrap();

    assert_eq!(restored.name.get(), original.name.get());
    assert_eq!(restored.purpose.get(), original.purpose.get());
    assert_eq!(restored.description.get(), original.description.get());
    assert_eq!(restored.traits.get(), original.traits.get());
    assert_eq!(restored.entity_ref().id(), original.entity_ref().id());
}

#[test]
fn partial_serialize_deserialize_roundtrip_preserves_partial_state() {
    let original = partial_tracked_role();
    let json = serde_json::to_value(&original).unwrap();
    let restored: TrackedRole = serde_json::from_value(json).unwrap();

    assert_eq!(restored.name.get(), Some(&"Engineering Lead".to_string()));
    assert!(restored.purpose.get().is_none(), "uninitialized field stays uninitialized");
}

// --- EntityRef serde ---

#[test]
fn entity_ref_serializes_with_id_and_kind() {
    let r: EntityRef<Role> = EntityRef::new("eng-lead");
    let json = serde_json::to_value(&r).unwrap();
    assert_eq!(json.get("id").and_then(|v| v.as_str()), Some("eng-lead"));
    assert_eq!(json.get("kind").and_then(|v| v.as_str()), Some("Role"));
}

#[test]
fn entity_ref_deserializes_from_id_and_kind() {
    let json = serde_json::json!({ "id": "eng-lead", "kind": "Role" });
    let r: EntityRef<Role> = serde_json::from_value(json).unwrap();
    assert_eq!(r.id(), "eng-lead");
}
```

---

## Implementation Notes

### `EntityRef` Serialize/Deserialize

`EntityRef<T, P>` must also be serializable for the entity_ref field. The serialized form is:
```json
{ "id": "eng-lead", "kind": "Role" }
```

Add `Serialize` and `Deserialize` impls to `EntityRef<T, P>` in `src/entity.rs`:
- Serialize: emit `{ "id": self.id, "kind": T::KIND }` (use `EntityKind` string representation)
- Deserialize: verify `"kind"` matches `T::KIND`, then construct `EntityRef::new(id)`

`EntityKind` needs a string representation. Add `Display` impl or a `as_str()` method:
```rust
impl EntityKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            EntityKind::Role => "Role",
            EntityKind::Hook => "Hook",
            // ...
        }
    }
}
```

### `WorkflowParent` in `EntityRef` Deserialize

For embedded entities (`Task`, `Relay`, `EmbeddedWorkflow`), `EntityRef<T, WorkflowParent>` needs the parent workflow id. The serialized form should include the parent:
```json
{ "id": "WriteProposal", "kind": "Task", "workflow_id": "InitiativeWorkflow" }
```

The `Deserialize` impl reads `workflow_id` to construct `WorkflowParent { workflow_id }`.

### `#[derive(Entity)]` Macro Extension

The `Serialize` and `Deserialize` impls are generated by the proc macro, not hand-written. The macro iterates domain fields and generates:
- The serialize `if let Some(v) = self.field.get()` blocks
- The deserialize field accumulator pattern with `initialize()` calls
- Special handling for `extensions` (flatten) and `entity_ref` (required field, not TrackedField)

### `serde` Dependency

Ensure in `Cargo.toml`:
```toml
[dependencies]
serde = { version = "1", features = ["derive"] }
serde_json = "1"
```

### Concurrent Write Safety in Deserialize

`initialize()` is idempotent via `OnceLock::set`. If two tasks concurrently deserialize the same entity, the first write wins — subsequent writes are silently discarded. This is correct behavior.

### Optional Field Deserialize

`description: Option<String>` means:
- JSON has `"description": null` → initialized with `None`
- JSON has `"description": "text"` → initialized with `Some("text")`
- JSON missing `"description"` key entirely → uninitialized (field never set)

The accumulator variable is `Option<Option<String>>`:
- `None` = key not present = uninitialized after deserialization
- `Some(None)` = `"description": null` = initialized with `None`
- `Some(Some("text"))` = `"description": "text"` = initialized with `Some("text")`

---

## Acceptance Criteria

- `cargo test tracked_serde` passes — all tests in `tests/tracked_serde.rs` green
- Fully-populated tracked entity serializes all initialized fields
- Partially-populated tracked entity skips uninitialized fields (absent, not null)
- Extensions map keys are flattened into the top-level object
- Deserialized fields are not dirty
- `entity_ref` is required; deserialization fails if absent
- Absent fields after deserialization remain uninitialized (not defaulted)
- Full serialize → deserialize roundtrip produces identical values
- Partial serialize → deserialize roundtrip preserves partial state
- Task 01-07 tests still pass
