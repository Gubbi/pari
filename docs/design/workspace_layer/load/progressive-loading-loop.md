# progressive-loading-loop

**Workspace Layer → `workspace_layer/load/`**

---

## Purpose

Loading is multi-round. Each round fetches one asset (or one field group), validates the result, and resolves any newly discovered cross-entity refs. Subsequent rounds may depend on data loaded in earlier rounds. Validation between rounds ensures unvalidated data never drives subsequent loads.

---

## Loop Structure

```
load(any_ref, requested_fields):

  Round 1: prerequisites
    For each field in requested_fields:
      If prerequisites not loaded → fetch prerequisite asset
      Expand refs → validate → merge

  Round 2: requested fields
    Fetch the asset(s) containing requested_fields
    Expand refs → validate → merge

  Done: all requested_fields now have initialized OnceLock
```

Each round:
1. **Fetch** — substrate call for one asset (or subset of fields)
2. **Expand** — batch-resolve cross-entity refs visible in the fetch result
3. **Validate** — partial validation of loaded fields in the result
4. **Merge** — write-once merge into cached entity (only after validation passes)

---

## Validate-Each-Round Invariant

Validation runs after ref expansion but before merge, in every round. This guarantees: data returned from the substrate is structurally valid before it is merged into the cached entity or used to resolve path variables in the next round.

If validation fails at any round, the load errors with `LoadError::ValidationFailed`. The failing round is not merged into the store; only fields from earlier successful rounds remain loaded.

---

## Termination

The loop terminates when all requested fields are populated (`value` is `Some`). Because:
- `path_deps` form a DAG (no cycles)
- Each round fills at least one asset
- Assets are finite

Termination is guaranteed.

---

## Path Dependency Example

Suppose an entity has a single-field asset whose path depends on field `category` (e.g. `{base}/{id}/{category}/content.bin`):

```
entity.content().await  // transparent load on first access

  Round 1 — prerequisite
    load_strategy(Entity, "content") → prerequisites: ["category"]
    "category" not loaded → fetch ref_asset (README.md)
      fields returned: name, ..., category, ...
    expand refs → validate → merge

  Round 2 — dependent field
    category is now known: "reports"
    resolve path: "{base}/SomeEntity/reports/content.bin"
    fetch asset
      fields returned: content
    expand refs → validate → merge

  Done: content OnceLock initialized
```

---

## Relation to ensure_mutable

`ensure_mutable` (see [59](ensure-mutable.md)) calls into this same load machinery before a mutation. The difference: `ensure_mutable` ensures the asset containing the field is loaded, whereas the progressive load loop is the implementation that actually executes the load rounds.
