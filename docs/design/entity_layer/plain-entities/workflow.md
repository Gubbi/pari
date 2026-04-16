# workflow-plain

**Owning layer: `entity`**

---

## Purpose

`Workflow` is the top-level process entity. It owns an ordered steps sequence and is the root of a workflow execution tree. Related work units (`Task`, `Relay`, `EmbeddedWorkflow`) are separate entities whose relationship to this workflow is expressed through their `EntityRef<_, WorkflowParent>` values, not by nesting their definitions inside the workflow struct.

---

## Definition

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

---

## Fields

- `entity_ref` — carries the workflow's id (CamelCase) and kind; top-level entity, defaults to `NoParent`
- `name` — human-readable display name
- `description` — optional short summary
- `purpose` — describes the operational goal of this workflow
- `raci` — required; accountability for the workflow as a whole
- `states` — lifecycle states; must include at least one `Done` semantic state; `Reviewing` required when any `Review` step is present (see [17 · state-entries](../value-types/state-entries.md))
- `steps` — ordered execution sequence; work steps carry `EntityRef` to related work entities; review steps are inline barriers (see [22 · workflow-restructuring](workflow-restructuring.md))
- `intercepts` — optional lifecycle hooks keyed by `WorkflowTrigger` (see [18 · intercepts](../value-types/intercepts.md))
- `guidance` — optional freeform guidance for tooling or agents
- `extensions` — open-ended metadata; only `x-` prefixed keys are permitted (see [13 · extensions](../value-types/extensions.md))
