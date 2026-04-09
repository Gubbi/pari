# step-types

**Data Model → `data_model/plain-entities/`**

---

## Purpose

`Step` is a flat enum covering all step kinds in a workflow's `steps` map. Work steps (Task, Relay, EmbeddedWorkflow) carry an `EntityRef` to the entity they execute and an optional dependency list. Review steps are inline barriers with no entity reference.

---

## Definition

```rust
pub enum Step {
    Task {
        entity_ref: EntityRef<Task, WorkflowParent>,
        depends_on: Option<Vec<String>>,
    },
    Relay {
        entity_ref: EntityRef<Relay, WorkflowParent>,
        depends_on: Option<Vec<String>>,
    },
    EmbeddedWorkflow {
        entity_ref: EntityRef<EmbeddedWorkflow, WorkflowParent>,
        depends_on: Option<Vec<String>>,
    },
    Review {
        approver: Vec<EntityRef<Role>>,
        on_reject: String,
    },
}
```

---

## Work Step Variants

`Task`, `Relay`, and `EmbeddedWorkflow` variants carry an `EntityRef` to the entity they execute. The step id (the key in `steps: IndexMap<String, Step>`) identifies the step position; the `entity_ref` identifies the entity that executes at that step.

The referenced entity is a separate store entry keyed by that full ref. The parent portion of the ref is what makes the step target part of this workflow tree.

`depends_on` lists the ids of other steps that must complete before this step can begin. Controls parallelism within a segment. Absent when the step has no dependencies.

---

## Review Variant

`Review` is a barrier — not a node in the dependency graph. Its position in the `steps` `IndexMap` defines a segment boundary (see [22 · workflow-restructuring](workflow-restructuring.md)).

- `approver` — one or more roles that must approve to pass the gate; role assignment to specific people happens at run time, not in the definition
- `on_reject` — id of the step to return to when the review is rejected
- No `depends_on` — barrier position in the ordered map defines sequencing; validation enforces this field is absent

---

## Relay Constraint

The `Relay` variant must not appear in a `ReusableWorkflow`'s steps, nor in any `EmbeddedWorkflow` within a `ReusableWorkflow`. Enforced by validation.
