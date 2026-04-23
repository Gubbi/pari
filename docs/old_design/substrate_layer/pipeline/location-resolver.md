# location-resolver-trait

**Owning layer: `substrate`**

---

## Purpose

`LocationResolver` translates a path template from `EntitySchema` into a concrete substrate location. Each substrate provides its own implementation. The resolver is entity-agnostic — it only sees a path template string and a serialized view of the entity.

---

## Trait

```rust
trait LocationResolver {
    type Location;

    fn resolve(
        &self,
        path_template: &str,
        entity: &serde_json::Value,
    ) -> Self::Location;

    fn base_of(&self, location: &Self::Location) -> String;
}
```

- `resolve` — expands template variables using values from the serialized entity; returns the concrete location for the asset
- `base_of` — returns the "base" portion of a location, used to resolve `{parent.base}` in child entity path templates

---

## Template Variables

| Variable | Source | Allowed in |
|---|---|---|
| `{id}` | `entity_ref.id()` (from serialized entity) | RefAssetDef, AssetDef |
| `{parent.base}` | `base_of(parent's ref_asset location)` | RefAssetDef, AssetDef |
| `{field_name}` | field value from entity (e.g. `artifact.name`) | AssetDef only |

`{parent.base}` is resolved by looking up the parent entity's ref_asset location and calling `base_of` on it. The parent entity must already be in the store (it is a prerequisite of any child entity).

---

## Entity Serialization

The entity is passed as `serde_json::Value` — the resolver sees a JSON object with field names as keys. This keeps the resolver entity-agnostic. The serialization is OnceLock-aware: uninitialized fields are skipped.

---

## RepoSubstrate Implementation

```rust
struct RepoLocationResolver {
    root: PathBuf,
}

impl LocationResolver for RepoLocationResolver {
    type Location = PathBuf;

    fn resolve(&self, path_template: &str, entity: &serde_json::Value) -> PathBuf {
        // expand {id}, {parent.base}, {field.nested} from entity JSON
        // prepend self.root
    }

    fn base_of(&self, location: &PathBuf) -> String {
        location.parent().unwrap().to_string_lossy().into()
    }
}
```

See [77 · repo-location-resolver](../repo-substrate/repo-location-resolver.md) for the full implementation.
