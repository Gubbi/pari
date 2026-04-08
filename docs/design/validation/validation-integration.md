# validation-integration

**Validation → `validation/`**

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

```
EntityServer::load(entity_ref, fields):
  1. substrate.load(entity_ref, fields) → partial TrackedEntity
  2. all_refs(partial)                  → collect all EntityRefs in the loaded fields
  3. batch substrate.exists(refs)       → insert stubs for any ref not in store
  4. run_validations(partial, fields: loaded_fields, kinds: &[Structural, Semantic, CrossEntity])
     → Err(LoadError::ValidationFailed) if non-empty
  5. merge partial into cached entity with dirty = false
```

The `all_refs` + batch pre-fetch ensures `has_ref` calls during `CrossEntity` validation are store-hits, avoiding N serial substrate round-trips. See [34 · all-refs](../data_model/tracked-entity/all-refs.md).

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
