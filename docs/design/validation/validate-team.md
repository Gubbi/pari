# validate-team

**Validation → `validation/`**

---

## Purpose

Validation for `Team`. Structural checks on fields, cross-entity ref existence on role and team refs, and cycle detection over the `include`/`import` composition graph.

---

## Schema

Wired as `Team::VALIDATION_SCHEMA` via `#[derive(Entity)]`.

```rust
static TEAM_VALIDATION_SCHEMA: ValidationSchema = ValidationSchema {
    structural: {
        "entity_ref": [kebab_case_id],
        "name":       [non_empty_str],
        "members":    [unique_member_handles],
        "extensions": [x_prefix_keys],
    },
    semantic:     {},
    cross_entity: {
        "members": [member_roles_exist],
        "include": [include_teams_exist, include_roles_exist, no_include_cycle],
        "import":  [import_teams_exist, no_import_cycle],
    },
};
```

---

## Structural Rules

`unique_member_handles` — checks `handle` uniqueness across all `members`; returns a `RuleViolation` with `sub_path: None` if duplicates exist.

---

## Cross-Entity Rules

### `members` rules

`member_roles_exist` — for each `member`, checks that `member.role` ref exists via `ref_exists`; `sub_path: Some("[{i}].role")` for each missing ref.

### `include` rules

`include_teams_exist` — for each key in `include`, checks that the team ref exists; `sub_path: Some("[{i}]")` for each missing team.

`include_roles_exist` — for each entry in `include`, checks that the role value ref exists; `sub_path: Some("[{i}].role")` for each missing role.

`no_include_cycle` — detects transitive cycles through `include` keys. See cycle detection below; reports at `sub_path: None`.

### `import` rules

`import_teams_exist` — for each entry in `import`, checks that the team ref exists; `sub_path: Some("[{i}]")` for each missing team.

`no_import_cycle` — detects transitive cycles through `import` entries. See cycle detection below; reports at `sub_path: None`.

---

## Cycle Detection

`include` and `import` form a directed composition graph. A team must not transitively include or import itself.

Both `no_include_cycle` and `no_import_cycle` use the same BFS algorithm, scoped to their respective edge kind:

```
detect_cycle(team, edge_fn):
  visited = {}
  queue  = edge_fn(team)           // direct include keys or import entries
  while queue not empty:
    ref = dequeue
    if ref == team.ref → cycle detected
    if ref in visited → skip
    visited.add(ref)
    ref_team = load(ref)
    enqueue edge_fn(ref_team)
```

A cycle error is reported once at the top-level field (`"include"` or `"import"`) with `sub_path: None`, not per-hop.
