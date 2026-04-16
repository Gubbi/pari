# state-entries

**Entity Layer → `entity_layer/value-types/`**

---

## Purpose

`WorkflowStateEntry` and `TaskStateEntry` define the lifecycle states an entity can be in. Each entity declares its own state list; the semantic variants are type-constrained per entity type.

---

## Definitions

```rust
pub struct WorkflowStateEntry {
    pub id: String,           // CamelCase, e.g. "UnderReview", "Done"
    pub description: String,
    pub semantic: Option<WorkflowSemantic>,
}

pub enum WorkflowSemantic {
    Reviewing,
    Done,
    Blocked,
    Failed,
}

pub struct TaskStateEntry {
    pub id: String,           // CamelCase, e.g. "Draft", "Done"
    pub description: String,
    pub semantic: Option<TaskSemantic>,
}

pub enum TaskSemantic {
    Done,
    Blocked,
    Failed,
}
```

---

## State Id Format

State ids are CamelCase (e.g. `Draft`, `UnderReview`, `Done`). Enforced by validation — a kebab-case id like `under-review` is rejected.

---

## Semantic Variants

Semantics are optional — most states have none. When present, they signal machine-readable lifecycle meaning:

| Semantic | Applies to | Meaning |
|---|---|---|
| `Done` | Workflow, Task | Terminal success state |
| `Reviewing` | Workflow only | Awaiting a review gate decision |
| `Blocked` | Workflow, Task | Stalled, waiting on external factor |
| `Failed` | Workflow, Task | Terminal failure state |

The separate enums enforce at the type level that `Reviewing` cannot appear on a `TaskStateEntry` — tasks are atomic work units, not process containers with review gates. This is a compile-time constraint, not just validation.

---

## Constraints (enforced by validation)

**Minimum 2 states** — every workflow and task must declare at least two states. A single-state lifecycle is not meaningful.

**At least one `Done` state** — every state list must include at least one state with `semantic: Done`.

**At least one non-`Done` state** — the state list must also include at least one state without `semantic: Done`. An entity where every state is `Done` is not meaningful.

**Workflow: `Reviewing` required when ReviewSteps present** — if a Workflow's steps contain at least one `ReviewStep`, the states list must include at least one state with `semantic: Reviewing`. A workflow with review gates but no reviewing state is a validation error. Workflows with only `WorkStep`s do not require it.
