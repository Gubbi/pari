# slot-and-asset-kind

**Owning layer: `substrate`**

---

## Purpose

`Slot` and `AssetKind` are the two substrate-defined primitives that the pipeline framework uses to remain entity-agnostic. `Slot` names where a field lives within an encoded asset. `AssetKind` declares an asset's capabilities.

---

## Slot

```rust
trait Slot: 'static {}
```

A `Slot` is a substrate-defined enum of encoding targets within an asset. Each substrate defines its own `Slot` enum; the framework trait has no variants of its own.

`'static` bound is required because slot values appear in `&'static [FieldMapping<S>]` inside `EntitySchema` — the schema is a compile-time constant.

The Codec receives slot values to drive encode/decode. The schema maps each entity field to a slot; the Codec handles the slot-to-storage-location translation. Entity awareness lives entirely in the schema — the Codec itself is entity-agnostic.

---

## AssetKind

```rust
struct AssetKind {
    distinguishes_create: bool,
    supports_partial: bool,
}
```

- `distinguishes_create` — `true` if the substrate distinguishes "create new" (`Post`) from "create-or-replace" (`Put`). `false` means all writes use `Put`.
- `supports_partial` — `true` if the substrate can write a subset of fields in an asset without reading and rewriting the full asset. Affects `mutable_without_load` in `LoadStrategy`.

Defined as `&'static AssetKind` constants per substrate, referenced from asset definitions in `EntitySchema`.

---

## Op Matrix

```
AssetKind capability     →  ops used
distinguishes_create:
  true                   →  Post (new) / Put (existing)
  false                  →  Put always
supports_partial:
  true                   →  Patch for dirty-fields-only writes
  false                  →  Put always (full rewrite)
```

---

## RepoSubstrate Constants

```rust
const MARKDOWN_FILE: AssetKind = AssetKind {
    distinguishes_create: false,
    supports_partial: false,
};

const RAW_FILE: AssetKind = AssetKind {
    distinguishes_create: false,
    supports_partial: false,
};
```
All Repo assets are full-rewrite. Every write is `Put`. See [76 · repo-asset-kinds](../repo-substrate/repo-asset-kinds.md).

Template files (`RAW_FILE`) are not a special case for `supports_partial`. `supports_partial` governs whether a subset of fields within a multi-field asset can be written independently — not relevant for single-field assets. The fact that a template file can be overwritten without loading its existing content is captured separately via `mutable_without_load: true` in the field's `LoadStrategy`. Prerequisites on `artifact` (for path resolution) are still required, but the file content itself is never read before the write. See [59 · ensure-mutable](../../workspace_layer/load/ensure-mutable.md).
