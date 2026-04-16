# validate-workflow

**Validation Layer → `validation_layer/`**

---

## Purpose

Validation for `Workflow`, `ReusableWorkflow`, and `EmbeddedWorkflow`. All three share structural and cross-entity rules via a common schema; `ReusableWorkflow` adds a constraint against `Relay` work units in its step tree.

---

## Shared Schema

```rust
static WORKFLOW_VALIDATION_SCHEMA: ValidationSchema = ValidationSchema {
    structural: {
        "entity_ref": [camel_case_id],
        "name":       [non_empty_str],
        "purpose":    [non_empty_str],
        "raci":       [raci_structural],
        "states":     [states_valid],
        "extensions": [x_prefix_keys],
    },
    semantic: {
        "steps": [depends_on_valid, on_reject_valid, reviewing_state_required],
    },
    cross_entity: {
        "raci":       [raci_roles_exist],
        "steps":      [work_step_refs_exist, review_approver_roles_exist],
        "intercepts": [intercept_hooks_exist, intercept_inputs_valid],
    },
};
```

`raci_structural` checks `raci.responsible` is non-empty (when `raci` is present — optional on `EmbeddedWorkflow`).

---

## Semantic Rules

All semantic rules receive the full entity and cross-reference fields within it.

`depends_on_valid` — for each work step with `depends_on`, every listed id must be a key in `steps` that appears before this step in the `IndexMap` order. `sub_path: Some("{step_id}.depends_on[{i}]")` for each invalid reference.

`on_reject_valid` — for each `Review` step, `on_reject` must be a key in `steps`. `sub_path: Some("{step_id}.on_reject")` for each invalid reference.

`reviewing_state_required` — if any `Review` step is present in `steps`, `states` must contain at least one state with `semantic: Reviewing`. `sub_path: None` on the `steps` field — the violation is at the steps level, not a single step.

---

## Cross-Entity Rules

`work_step_refs_exist` — for each `Task`, `Relay`, or `EmbeddedWorkflow` step variant, checks the `entity_ref` exists via `ref_exists`; `sub_path: Some("{step_id}.entity_ref")` for each missing ref.

`review_approver_roles_exist` — for each `Review` step, checks each role ref in `approver` via `all_refs_exist`; `sub_path: Some("{step_id}.approver[{i}]")` for each missing ref.

`raci_roles_exist` — shared primitive; checks all role refs in `raci`. See [83 · validate-shared](validate-shared.md).

`intercept_hooks_exist` and `intercept_inputs_valid` — shared primitives. See [83 · validate-shared](validate-shared.md).

---

## ReusableWorkflow — Additional Constraint

`ReusableWorkflow` adds one cross-entity rule to its schema: `no_relay_in_tree`.

```rust
static REUSABLE_WORKFLOW_VALIDATION_SCHEMA: ValidationSchema = ValidationSchema {
    structural:   { /* same as WORKFLOW_VALIDATION_SCHEMA */ },
    semantic:     { /* same as WORKFLOW_VALIDATION_SCHEMA */ },
    cross_entity: {
        /* all from WORKFLOW_VALIDATION_SCHEMA, plus: */
        "steps": [work_step_refs_exist, review_approver_roles_exist, no_relay_in_tree],
    },
};
```

`no_relay_in_tree` — BFS over all work step refs in `steps` (and recursively into any `EmbeddedWorkflow` steps). If any `Relay` entity is found anywhere in the tree, returns a `RuleViolation` with `sub_path: None`. A single error is reported at the `steps` field regardless of how deep the relay is.

Each type exposes its schema as `Entity::VALIDATION_SCHEMA` via `#[derive(Entity)]`: `Workflow` and `EmbeddedWorkflow` use `WORKFLOW_VALIDATION_SCHEMA`; `ReusableWorkflow` uses `REUSABLE_WORKFLOW_VALIDATION_SCHEMA`.
