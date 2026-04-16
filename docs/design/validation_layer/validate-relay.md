# validate-relay

**Validation Layer → `validation_layer/`**

---

## Purpose

Validation for `Relay`. Structural checks on identity and `state_map`, semantic check on `state_map` keys, and cross-entity checks on the `delegates_to` ref, `state_map` target state ids, raci role refs, and hook refs in `intercepts`.

---

## Schema

Wired as `Relay::VALIDATION_SCHEMA` via `#[derive(Entity)]`.

```rust
static RELAY_VALIDATION_SCHEMA: ValidationSchema = ValidationSchema {
    structural: {
        "entity_ref": [camel_case_id],
        "name":       [non_empty_str],
        "purpose":    [non_empty_str],
        "raci":       [raci_structural],
        "state_map":  [non_empty_map, camel_case_state_keys],
        "extensions": [x_prefix_keys],
    },
    semantic: {},
    cross_entity: {
        "delegates_to": [delegates_to_exists],
        "state_map":    [maps_to_states_exist],
        "raci":         [raci_roles_exist],
        "intercepts":   [intercept_hooks_exist, intercept_inputs_valid],
    },
};
```

---

## Structural Rules

`non_empty_map` on `state_map` — the relay must declare at least one state; returns a `RuleViolation` with `sub_path: None` if the map is empty.

`camel_case_state_keys` — checks each key in `state_map` against the `camel_case` primitive; `sub_path: Some("{key}")` for each non-CamelCase key.

`raci_structural` — shared primitive; checks `raci.responsible` is non-empty (when `raci` is present). See [83 · validate-shared](validate-shared.md).

---

## Cross-Entity Rules

`delegates_to_exists` — checks the `delegates_to` ref via `ref_exists`; the referenced `ReusableWorkflow` must exist in the store.

`maps_to_states_exist` — loads the `delegates_to` `ReusableWorkflow` and checks that each `StateMapEntry.maps_to` value is a state id declared in that workflow's `states`; `sub_path: Some("{key}.maps_to")` for each unresolved target.

`raci_roles_exist` — shared primitive; checks all role refs in `raci`. See [83 · validate-shared](validate-shared.md).

`intercept_hooks_exist` and `intercept_inputs_valid` — shared primitives; check hook ref existence and input binding for each `HookCall` in `intercepts`. See [83 · validate-shared](validate-shared.md).
