# validate-role

**Owning layer: `validation`**

---

## Purpose

Validation for `Role`. Structural only — `Role` is a leaf entity with no cross-entity references and no entity-local semantic rules.

---

## Schema

Wired as `Role::VALIDATION_SCHEMA` via `#[derive(Entity)]`.

```rust
static ROLE_VALIDATION_SCHEMA: ValidationSchema = ValidationSchema {
    structural: {
        "entity_ref":  [kebab_case_id],
        "name":        [non_empty_str],
        "description": [opt_non_empty_str],
        "purpose":     [non_empty_str],
        "traits":      [each_item_non_empty],
        "extensions":  [x_prefix_keys],
    },
    semantic:     {},
    cross_entity: {},
};
```

No semantic rules — all constraints are purely structural.
No cross-entity rules — `Role` holds no refs to other entities.
