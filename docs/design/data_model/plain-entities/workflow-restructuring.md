# workflow-restructuring

**Data Model → `data_model/plain-entities/`**

---

## Purpose

Workflow structure is a single ordered `steps` sequence. Work units — `Task`, `Relay`, `EmbeddedWorkflow` — carry their own `EntityRef<T, WorkflowParent>`, which is what ties them into the workflow tree. The steps sequence references them by `EntityRef`; it does not embed their definitions or maintain a separate `definitions` map.

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

## Related Work Entities

`Task`, `Relay`, and `EmbeddedWorkflow` are not embedded in the workflow struct. Each carries `entity_ref: EntityRef<T, WorkflowParent>` that identifies where it sits in the workflow tree. In the Store they are still standalone entries indexed by full `AnyEntityRef`, so hierarchy lives in identity rather than in nested storage.

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
