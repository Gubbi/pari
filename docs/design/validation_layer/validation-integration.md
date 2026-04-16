# validation-integration

**Validation Layer → `validation_layer/`**

---

## Purpose

Describes how validation wires into `EntityServer` at each of the three trigger points: setter, load path, and check-in. Covers the call sites, the `fields` and `kinds` arguments passed to `run_validations`, and how errors are surfaced.

---

## Setter

```
setter(value) -> Result<(), SetterError>:
  1. ensure_mutable()           → Err(SetterError::Substrate) on failure
  2. run_validations(entity, fields: &[field_name], kinds: &[Structural, Semantic])
     → Err(SetterError::Validation) if non-empty
  3. write value into field
```

Setters run only `Structural` and `Semantic` — no store access. The `fields` slice contains exactly the one field being set.

---

## Load Path

The authoritative load sequence lives in [store-load-internal](../workspace_layer/load/store-load-internal.md). From the validation layer's perspective, load integration is:

- `EntityServer` fetches partial fields from the substrate
- it may prefetch refs via `all_refs()` + batched `exists()` as an optimization
- `run_validations(..., kinds: &[Structural, Semantic, CrossEntity])` runs before merge
- `LoadError::ValidationFailed` aborts the merge for that fetch result

This doc intentionally does not restate the detailed load algorithm; see [store-load-internal](../workspace_layer/load/store-load-internal.md) and [ensure-mutable](../workspace_layer/load/ensure-mutable.md) for the canonical flow.

---

## Check-In

```
EntityServer::commit(entity):
  // New entity — full validation
  if entity is new:
    run_validations(entity, fields: &[], kinds: &[Structural, Semantic, CrossEntity])
    → Err(CommitError::ValidationFailed) if non-empty

  // Modified entity — cross-entity only on dirty fields
  if entity is modified:
    run_validations(entity, fields: dirty_fields, kinds: &[CrossEntity])
    → Err(CommitError::ValidationFailed) if non-empty

  // Gate passed — proceed with persist
```

For modified entities, `#1` and `#2` are not re-run: structural values are immutable between setter and check-in, and semantic rules were fully enforced by setters — because tracked entity accessors transparently load uninitialized fields, ensuring any sibling fields a semantic rule needed were fetched at setter time. Only `#3` re-runs: setters do not have store access, so cross-entity validation was never run at setter time. Additionally, store state may have changed since the setter ran.

`fields: &[]` passed for new entities instructs `run_validations` to run all fields in the schema.

---

## Error Surfaces

| Trigger | Error type | Caller receives |
|---|---|---|
| Setter | `SetterError::Validation(ValidationErrors)` | field not written |
| Load path | `LoadError::ValidationFailed(ValidationErrors)` | merge not applied |
| Check-in (new or modified) | `CommitError::ValidationFailed(ValidationErrors)` | entity stays checked out |
