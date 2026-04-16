# validate-hook

**Owning layer: `validation`**

---

## Purpose

Validation for `Hook`. Structural only — `Hook` is a leaf entity with no cross-entity references and no entity-local semantic rules beyond structure.

---

## Schema

Wired as `Hook::VALIDATION_SCHEMA` via `#[derive(Entity)]`.

```rust
static HOOK_VALIDATION_SCHEMA: ValidationSchema = ValidationSchema {
    structural: {
        "entity_ref":  [kebab_case_id],
        "name":        [non_empty_str],
        "description": [non_empty_str],
        "inputs":      [each_name_non_empty, each_description_non_empty, unique_input_names],
        "extensions":  [x_prefix_keys],
    },
    semantic:     {},
    cross_entity: {},
};
```

### `inputs` rules

`each_name_non_empty` — iterates `inputs`, returns a `RuleViolation` with `sub_path: Some("[{i}].name")` for each entry where `name` is empty.

`each_description_non_empty` — same pattern, `sub_path: Some("[{i}].description")` for each empty `description`.

`unique_input_names` — checks `name` uniqueness across all inputs; returns a `RuleViolation` with `sub_path: None` if duplicates exist.

No semantic rules — all constraints are purely structural.
No cross-entity rules — `Hook` holds no refs to other entities.
