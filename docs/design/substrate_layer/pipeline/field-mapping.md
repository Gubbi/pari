# field-mapping-and-asset-defs

**Owning layer: `substrate`**

---

## Purpose

`FieldMapping`, `RefAssetDef`, and `AssetDef` are the building blocks of `EntitySchema`. Together they declare how an entity's fields map to storage locations — without any per-entity code in the pipeline.

---

## FieldMapping

```rust
struct FieldMapping<S: Slot> {
    key: &'static str,   // field name, matches TrackedEntity field
    slot: S,             // where in the asset this field lives
}
```

Maps a single field to a slot within an asset. The Codec uses `slot` to determine how to encode/decode the field value. The `key` matches the field name used in serde and dirty tracking.

---

## RefAssetDef

```rust
struct RefAssetDef<S: Slot> {
    path_template: &'static str,
    kind: &'static AssetKind,
    fields: &'static [FieldMapping<S>],
}
```

The **primary asset** for an entity — the one whose path is determined solely from the entity's `EntityRef` (id + parent chain). No field values are needed for path resolution.

- `path_template` — template with `{id}` and `{parent.base}` variables; no `{field_name}` allowed
- `kind` — references an `AssetKind` constant for this substrate
- `fields` — all fields that live in this asset

The ref_asset always exists for a loadable entity. It is always the first asset fetched.

---

## AssetDef

```rust
struct AssetDef<S: Slot> {
    path_template: &'static str,
    kind: &'static AssetKind,
    fields: &'static [FieldMapping<S>],
    path_deps: &'static [&'static str],
}
```

An **asset** — stores fields that cannot be co-located in the ref_asset. Its path may optionally depend on field values from the ref_asset via `path_deps`.

- `path_template` — may include `{field_name}` variables whose values come from the ref_asset
- `path_deps` — field names that must be loaded (from the ref_asset) before this asset's path can be resolved; these become `LoadStrategy.prerequisites` for any field in this asset

Dependencies form a DAG rooted at the ref_asset. No cycles are possible:
- The ref_asset has no field dependencies (it is always resolved first)
- An asset's `path_deps` may only reference fields in the ref_asset (not other assets)

---

## Example — Task

```
ref_asset:
  path_template: "{parent.base}/{id}/README.md"
  kind: &MARKDOWN_FILE
  fields: [name, purpose, instructions, criteria, artifact, states, hooks, guidance, extensions]

assets:
  [{
    path_template: "{parent.base}/{id}/template.md"
    kind: &RAW_FILE
    fields: [template_content]
  }]
```

`template_content` is a single-field asset with no path dependencies. This produces:

```rust
load_strategy(Task, "template_content") → LoadStrategy {
    prerequisites: &[],
    mutable_without_load: true,   // single-field asset; full overwrite, no prior read needed
}
```
