# load-strategy

**Owning layer: `substrate`**

---

## Purpose

`LoadStrategy` is a static query that answers two questions for any (entity kind, field name) pair: what must be loaded before this field can be loaded, and can this field be mutated without loading the containing asset first?

Because both answers are derivable from the static `EntitySchema`, `load_strategy` is an associated function — no substrate instance required.

---

## Definition

```rust
struct LoadStrategy {
    prerequisites: &'static [&'static str],
    mutable_without_load: bool,
}
```

- `prerequisites` — field names that must be loaded before this field can be loaded. Forms a DAG rooted at fields with no prerequisites. `EntityServer`'s load handler resolves these recursively before issuing a substrate call.
- `mutable_without_load` — `true` if the field can be set without loading its containing asset first. Derived from the asset: `true` when `supports_partial: true` (fields can be written independently) or when the asset has exactly one field (full overwrite requires no prior read). `false` for multi-field full-rewrite assets.

---

## Associated Function on Substrate

```rust
trait Substrate {
    fn load_strategy(entity_kind: EntityKind, field: &str) -> LoadStrategy;
    // ...
}
```

No `&self` — the answer does not depend on runtime substrate state. It is derivable from the entity's `EntitySchema` for this substrate. The substrate impl returns values directly from the static schema.

---

## Examples

**Single-field asset (no prerequisites):**
```
field: "content"  // sole field of a RAW_FILE asset; path has no dependencies
  prerequisites: []
  mutable_without_load: true   // single-field asset; full overwrite, no prior read needed
```

**Single-field asset with path dependency (hypothetical):**
```
field: "attachment"  // sole field of an asset whose path includes {category}
  prerequisites: ["category"]  // category must be loaded first to resolve the path
  mutable_without_load: true   // still single-field; once path is resolved, no prior read needed
```

**Multi-field asset:**
```
field: "name"  // one of several fields in a shared MARKDOWN_FILE ref_asset
  prerequisites: []
  mutable_without_load: false  // ref_asset has multiple fields; full-file rewrite requires all loaded
```

For a substrate with `supports_partial: true` (e.g. DynamoDB), all fields get `mutable_without_load: true` regardless of how many fields share the asset.

---

## Where Used

- `EntityServer`'s load handler — calls `load_strategy` per field to determine prerequisite ordering before issuing substrate fetch calls
- `ensure_mutable()` — calls `load_strategy` in `set_*()` accessors to decide whether to trigger a load before mutation
