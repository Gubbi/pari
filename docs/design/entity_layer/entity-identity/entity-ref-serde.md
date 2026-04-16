# entity-ref-serde

**Entity Layer → `entity_layer/entity-identity/`**

---

## Purpose

`EntityRef` serializes to a self-describing wire format that includes the entity kind, id, and optionally the parent ref. On deserialization, the kind is validated against the expected type — mismatches are errors.

---

## Wire Format

```json
// EntityRef<Role, NoParent>
{"id": "eng-lead", "kind": "Role"}

// EntityRef<Task, EntityRef<Workflow, NoParent>>
{
  "id": "WriteProposal",
  "kind": "Task",
  "parent": {"id": "Initiative", "kind": "Workflow"}
}
```

Fields:
- `id` — the entity's identifier
- `kind` — the `EntityKind` string tag (`#[serde(rename_all = "PascalCase")]` gives `"Role"`, `"Task"`, etc.)
- `parent` — absent for `NoParent`; present and recursive for embedded entities

---

## Serialize

```rust
impl<T: Entity, P: ParentKind + Serialize> Serialize for EntityRef<T, P> {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let mut map = s.serialize_map(None)?;
        map.serialize_entry("id", &self.id)?;
        map.serialize_entry("kind", &T::KIND)?;  // T::KIND baked in at compile time
        if /* P is not NoParent */ {
            map.serialize_entry("parent", &self.parent)?;
        }
        map.end()
    }
}
```

`T::KIND` is available at compile time via monomorphization — no stored field needed. `NoParent` serializes as absent (no `parent` key).

---

## Deserialize

```rust
impl<'de, T: Entity, P: ParentKind + Deserialize<'de>> Deserialize<'de> for EntityRef<T, P> {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        // 1. Read `kind` field, parse as EntityKind
        // 2. Validate: kind == T::KIND — error if mismatch
        // 3. Read `id`
        // 4. Deserialize `parent` (absent → NoParent)
    }
}
```

A missing `kind` field is an error — the wire format is self-describing by design. Silently defaulting to `T::KIND` would defeat validation.

Kind mismatch example — error:
```json
{"id": "eng-lead", "kind": "Hook"}
// deserialized into EntityRef<Role> → error: expected "Role", got "Hook"
```

---

## Parent Serialization

`NoParent` serializes as absent — the `parent` key is omitted entirely. An embedded entity's parent is serialized recursively using the same wire format, producing a nested object.
