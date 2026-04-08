# workflow-variants

**Data Model → `data_model/plain-entities/`**

---

## Purpose

`ReusableWorkflow` and `EmbeddedWorkflow` are the other two workflow variants. All three share the same structural shape — `steps` sequence only — but differ in entity identity, parent kind, and constraints. Work units are standalone entities checked in alongside the workflow.

---

## ReusableWorkflow

A top-level reusable subprocess. Referenced by `Relay` entities as the target to delegate to.

```rust
pub struct ReusableWorkflow {
    pub entity_ref: EntityRef<ReusableWorkflow>,
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

`raci` required. `Relay` must not appear in any work entity checked in under a `ReusableWorkflow` — enforced by cross-entity validation at check-in.

---

## EmbeddedWorkflow

A subprocess associated with a parent workflow via `EntityRef<EmbeddedWorkflow, WorkflowParent>`. Structurally identical to `Workflow` and `ReusableWorkflow` but carries a parent ref. Can nest recursively.

```rust
pub struct EmbeddedWorkflow {
    pub entity_ref: EntityRef<EmbeddedWorkflow, WorkflowParent>,
    pub name: String,
    pub description: Option<String>,
    pub purpose: String,
    pub raci: Option<Raci>,
    pub states: Vec<WorkflowStateEntry>,
    pub steps: IndexMap<String, Step>,
    pub intercepts: Option<HashMap<WorkflowTrigger, HookCall>>,
    pub guidance: Option<String>,
    pub extensions: Extensions,
}
```

`raci` optional — accountability can be inherited from the parent workflow. The `WorkflowParent` type handles arbitrary nesting depth (see [07 · parent-kind](../entity-identity/parent-kind.md)).

---

## Workflow Variant Comparison

| | `Workflow` | `ReusableWorkflow` | `EmbeddedWorkflow` |
|---|---|---|---|
| Parent | `NoParent` | `NoParent` | `WorkflowParent` |
| `raci` | required | required | optional |
| Relay constraint | none | no Relay in tree — cross-entity validation | no Relay if root is ReusableWorkflow — cross-entity validation |
