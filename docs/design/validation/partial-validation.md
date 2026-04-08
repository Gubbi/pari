# partial-validation

**Validation → `validation/`**

---

## Purpose

Describes how `run_validations` handles sparse entities and field subsets — validating only the fields that are present, without requiring the full entity to be loaded.

---

## Field Subset Selection

`run_validations` accepts a `fields: &[&str]` argument:

- `&[]` — run all fields present in the schema
- `&["name", "states"]` — run only those fields

The runner intersects the requested field list with the schema maps. For each field:
- Look up the field in `schema.structural` — run any listed rules if `Structural` is in `kinds`
- Look up in `schema.semantic` — run any listed rules if `Semantic` is in `kinds`
- Look up in `schema.cross_entity` — run any listed rules if `CrossEntity` is in `kinds`

A field absent from a map simply has no rules of that kind — not an error.

---

## Sparse Entity Compatibility

Tracked entities can be partially loaded — only some fields may be initialized in their `OnceLock`. `run_validations` does not require the full entity to be pre-loaded. Rules are invoked only for fields explicitly listed (or all schema fields when `fields: &[]`).

At the load path, `fields` is set to the set of newly loaded fields. Rules for other fields are not run — they were already validated when those fields were first loaded.

At check-in for modified entities, `fields` is set to `dirty_fields()` — only the fields mutated since the last reset are re-validated.

---

## Rule Safety with Sparse State

Structural rules receive only the field value — they have no dependency on other entity fields, so sparse state is never an issue.

Semantic and cross-entity rules receive the tracked entity directly. A rule for field `X` may access sibling field `Y` via `entity.y().await`. If `Y` is not yet loaded, the async accessor transparently loads it via `OnceLock` before returning. Rules always receive fully resolved values — there is no "unset state" to handle.

---

## External API (Future)

An external API layer will run only `Structural` validations on user-supplied field values before any entity is created or mutated. It passes only the fields the API call touches:

```
run_validations(entity, fields: api_fields, kinds: &[Structural])
```

The three-kind split is what makes this possible without a separate code path.
