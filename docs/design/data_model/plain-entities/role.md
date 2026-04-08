# role-plain

**Data Model → `data_model/plain-entities/`**

---

## Purpose

`Role` is a top-level entity representing a named participant type in a workflow. It carries identity, descriptive fields, and optional capability tags.

---

## Definition

```rust
pub struct Role {
    pub entity_ref: EntityRef<Role>,
    pub name: String,
    pub description: Option<String>,
    pub purpose: String,
    pub traits: Option<Vec<String>>,
    pub extensions: Extensions,
}
```

---

## Fields

- `entity_ref` — carries the role's id and kind; `EntityRef<Role>` defaults to `NoParent` (top-level entity)
- `name` — human-readable display name
- `description` — optional short label; present when a brief summary separate from purpose is useful
- `purpose` — describes the operational function of this role in workflows
- `traits` — optional list of capability tags (e.g. `["reviewer", "approver"]`); used by tooling to filter or assign roles
- `extensions` — open-ended metadata; only `x-` prefixed keys are permitted (see [13 · extensions](../value-types/extensions.md))
