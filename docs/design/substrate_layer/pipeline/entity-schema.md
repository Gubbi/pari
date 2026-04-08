# entity-schema

**Substrate Layer → `substrate_layer/pipeline/`**

---

## Purpose

`EntitySchema<S>` is the complete declarative description of how one entity type maps to a substrate. One schema per (entity type, substrate). It is the single source of truth for both read and write pipelines.

---

## Definition

```rust
struct EntitySchema<S: Slot> {
    ref_asset: RefAssetDef<S>,
    assets: &'static [AssetDef<S>],
}
```

- `ref_asset` — the primary asset; path resolved from EntityRef only; always fetched first
- `assets` — zero or more additional assets; paths may optionally depend on field values from the ref_asset via `path_deps`

Most entities have only a `ref_asset` and no additional assets. Additional assets are declared when an entity's storage spans multiple files (e.g., Task's template file).

---

## Two Consumers

**AssetMapper (write path):** reads `ref_asset.path_template` and `assets[*].path_template` to determine which assets to write, and `fields` membership to decide which fields belong to each asset. Selects assets whose fields overlap with the entity's dirty fields.

**Codec:** reads `fields[*].slot` to drive encode/decode. Each field's slot tells the Codec exactly where to place or read the value within the encoded asset. No per-entity codec logic.

**LoadStrategy derivation:** `load_strategy(kind, field)` is derived from the schema:
- Find which asset contains `field`
- `prerequisites` = that asset's `path_deps`
- `mutable_without_load` = `asset.kind.supports_partial || asset.fields.len() == 1`

Single-field assets (`fields.len() == 1`) get `mutable_without_load: true` even when `supports_partial: false` — writing the sole field is a full-asset overwrite; no prior read is needed.

---

## Compile-Time Static

All schema values are `'static`. The schema is a compile-time constant — no heap allocation, no runtime schema loading. This is enforced by the `Slot: 'static` bound and the `&'static` slices.

```rust
impl SubstrateSchema<RepoSubstrate> for Role {
    const SCHEMA: EntitySchema<RepoSlot> = EntitySchema {
        ref_asset: RefAssetDef {
            path_template: "roles/{id}.md",
            kind: &MARKDOWN_FILE,
            fields: &[
                FieldMapping { key: "name",       slot: RepoSlot::H1 },
                FieldMapping { key: "purpose",    slot: RepoSlot::FrontmatterKey("purpose") },
                FieldMapping { key: "traits",     slot: RepoSlot::FrontmatterKey("traits") },
                FieldMapping { key: "extensions", slot: RepoSlot::FrontmatterFlattened },
            ],
        },
        assets: &[],
    };
}
```
