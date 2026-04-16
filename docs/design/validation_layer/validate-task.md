# validate-task

**Validation Layer → `validation_layer/`**

---

## Purpose

Validation for `Task`. Structural checks on fields, entity-local semantic checks on `states`, and cross-entity checks on role refs in `raci`, the `artifact.kind` ref, and hook refs in `intercepts`.

---

## Schema

Wired as `Task::VALIDATION_SCHEMA` via `#[derive(Entity)]`.

```rust
static TASK_VALIDATION_SCHEMA: ValidationSchema = ValidationSchema {
    structural: {
        "entity_ref":   [camel_case_id],
        "name":         [non_empty_str],
        "purpose":      [non_empty_str],
        "instructions": [non_empty_list, each_item_non_empty],
        "criteria":     [non_empty_list, each_item_non_empty],
        "raci":         [raci_structural],
        "states":       [states_valid],
        "extensions":   [x_prefix_keys],
    },
    semantic: {},
    cross_entity: {
        "raci":       [raci_roles_exist],
        "artifact":   [artifact_kind_exists],
        "intercepts": [intercept_hooks_exist, intercept_inputs_valid],
    },
};
```

---

## Structural Rules

`each_item_non_empty` on `instructions` and `criteria` — iterates the list; returns a `RuleViolation` with `sub_path: Some("[{i}]")` for each empty string entry.

`raci_structural` — shared primitive; checks `raci.responsible` is non-empty. See [83 · validate-shared](validate-shared.md).

`states_valid` — shared primitive; checks CamelCase ids, uniqueness, min 2, at least one Done, at least one non-Done.

---

## Cross-Entity Rules

`raci_roles_exist` — shared primitive; checks all role refs in `raci.responsible`, `raci.accountable`, `raci.consulted`, and `raci.informed` via `ref_exists`; `sub_path` set to the nested field path (e.g. `Some("responsible[0]")`).

`artifact_kind_exists` — checks `artifact.kind` ref via `ref_exists`; `sub_path: Some("kind")`.

`intercept_hooks_exist` — for each entry in `intercepts`, checks that the hook ref in the `HookCall` exists via `ref_exists`; `sub_path: Some("{trigger}.hook")` for each missing ref.

`intercept_inputs_valid` — for each entry in `intercepts`, loads the referenced hook and validates the `with` bindings against the hook's declared `inputs`: every key in `with` must match a declared `HookInput.name` (no unknown keys), and every declared input must have a binding (no missing keys). `sub_path: Some("{trigger}.with")` for binding violations. See [83 · validate-shared](validate-shared.md) for the shared `hook_call_inputs_valid` primitive.
