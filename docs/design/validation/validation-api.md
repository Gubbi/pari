# validation-api

**Validation → `validation/`**

---

## Principles

1. **Field-level granularity** — all validations are field-level. No validation requires the full entity to be present. Sparse entities are valid as long as their loaded fields pass.

2. **Completeness is type-enforced** — plain entity types require all mandatory fields at construction. `From<PlainEntity>` gives all fields populated. Post-conversion, tracked mutations are field-by-field; "required field missing" is not a runtime condition.

3. **Best-effort before check-in** — setters run structural and entity-local semantic validations proactively. Cross-entity validation requires store access and runs at load-time and check-in only.

4. **No skipping at check-in** — every field being checked in passes all three validation kinds. Cross-entity refs are verified via `has_ref`, which hits the substrate if necessary. Nothing enters the store without passing the full gate.

5. **Errors accumulate** — all failing validations are collected before returning; no short-circuit on first failure.

---

## Three Validation Kinds

| Kind | Description | Context needed |
|---|---|---|
| **Structural** | Value format, pattern, cardinality (e.g. kebab id, `states ≥ 2`) | Value only |
| **Entity-local semantic** | Domain rules using only the entity's own data (e.g. `depends_on` refs prior steps, Done state required) | Entity's current loaded state |
| **Cross-entity** | Ref existence, hook input binding against declared inputs | Store (via `has_ref`) |

---

## Three Validation Triggers

### 1 · Setter (best-effort)

After `ensure_mutable` and before writing the value, the setter runs structural (#1) and entity-local semantic (#2) validation on the incoming value in context of the entity's current loaded state. No store access — #3 is not run here.

A validation failure prevents the field from being set and is returned as `SetterError::Validation`.

### 2 · Load path (authoritative for loaded data)

The substrate returns a partial `TrackedEntity` with only the newly loaded fields. The substrate does not validate — `EntityServer`'s load handler validates before merge:

```
substrate.load() → partial TrackedEntity
  → all_refs() on result
  → batch substrate.exists() for refs not already in store → stubs inserted
  → validate #1 + #2 + #3 on loaded fields
      (has_ref calls are store-hits: stubs pre-seeded by the batch above)
  → Err(LoadError::ValidationFailed) if any fail
  → merge into cached entity with dirty=false only on success
```

The `all_refs()` + batch pre-fetch avoids N serial substrate round-trips during #3 validation — by the time `has_ref` calls run, all refs are already in the store as stubs. See [34 · all-refs](../data_model/tracked-entity/all-refs.md) and [58 · store-load-internal](../workspace_layer/load/store-load-internal.md).

### 3 · Check-in (authoritative gate)

Runs inside `EntityServer`'s commit handler before any data enters the store:

- **New entity**: #1 + #2 + #3 on all fields (`From<PlainEntity>` guarantees completeness)
- **Modified entity**: #3 only on dirty fields — setters do not have store access so cross-entity validation was never run at setter time; additionally, store state may have changed since the setter ran. #1 + #2 were fully enforced at setter time (transparent field loading via the tracked entity ensures semantic rules ran with complete context) and are not re-run

On any failure the commit is rejected, the entity stays checked out, and `ValidationErrors` is returned to the caller.

---

## Error Types

```rust
struct ValidationErrors {
    errors: Vec<FieldValidationError>,
}

struct FieldValidationError {
    path: String,           // dot-notation: "id", "steps.WriteProposal.depends_on"
    message: String,
    kind: ValidationKind,
}

enum ValidationKind { Structural, Semantic, CrossEntity }
```

Flat list — composable via `Vec::extend`. Each validator appends to the same collection.

---

## SetterError

```rust
enum SetterError {
    Substrate(SubstrateError),   // from ensure_mutable
    Validation(ValidationErrors), // from #1 + #2 validation
}
```

Setters return `Result<(), SetterError>`. See [async-accessor-variants](../codegen/async/async-accessor-variants.md).
