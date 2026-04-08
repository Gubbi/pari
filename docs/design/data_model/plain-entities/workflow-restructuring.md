# workflow-restructuring

**Data Model → `data_model/plain-entities/`**

---

## Purpose

Workflow structure is a single ordered `steps` sequence. Work units — `Task`, `Relay`, `EmbeddedWorkflow` — are standalone entities that carry their own `EntityRef<T, WorkflowParent>`. They are constructed independently and checked in alongside the workflow. The steps sequence references them by `EntityRef`; it does not embed their definitions.

---

## Structure

```rust
pub struct Workflow {
    pub entity_ref: EntityRef<Workflow>,
    pub name: String,
    pub description: Option<String>,
    pub purpose: String,
    pub raci: Raci,
    pub states: Vec<WorkflowStateEntry>,
    pub steps: IndexMap<String, Step>,
    pub intercepts: Option<HashMap<WorkflowTrigger, HookCall>>,
    pub guidance: Option<String>,
    pub extensions: Extensions,
}
```

`steps` — ordered sequence of work steps and review gates. Work steps carry an `EntityRef` to a standalone work entity; review steps are inline barriers with no associated entity. See [23 · step-types](step-types.md).

---

## Standalone Work Entities

`Task`, `Relay`, and `EmbeddedWorkflow` are not embedded in the workflow struct. Each carries `entity_ref: EntityRef<T, WorkflowParent>` that identifies which workflow it belongs to. Clients construct them independently and submit them together with the workflow at check-in. `EntityServer`'s commit handler validates the cross-entity links at that point.

---

## Review Gates as Segment Barriers

Review steps are not nodes in the dependency graph. Their position in the `steps` `IndexMap` defines a segment boundary — all work steps before the gate form one segment, all after form the next.

```
┌── segment 1 ───┐        ┌── segment 2 ───┐
│ WriteProposal  │        │ ExecuteJob ───┐│
│                ├─▶ Gate▶│               ││
│ WriteSpec ─────┘        │ RunTests ─────┘│
└────────────────┘        └────────────────┘
```

- Review gates have no `depends_on`
- Parallelism is expressed within a segment via `depends_on` on work steps
- A step that needs to participate in dependency ordering should be a Task, not a review gate
