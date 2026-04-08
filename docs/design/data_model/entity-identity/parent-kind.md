# parent-kind

**Data Model → `data_model/entity-identity/`**

---

## Purpose

`EntityRef<T, P>` carries a parent type parameter `P`. `ParentKind` is the trait bounding `P`, constraining which types are valid parents at compile time.

---

## `ParentKind` Trait and Implementors

```rust
pub trait ParentKind {}

/// For top-level entities with no parent.
pub struct NoParent;
impl ParentKind for NoParent {}

/// Concrete parent type for embedded entities (Task, Relay, EmbeddedWorkflow).
pub enum WorkflowParent {
    Workflow(EntityRef<Workflow>),
    ReusableWorkflow(EntityRef<ReusableWorkflow>),
    EmbeddedWorkflow(Box<EntityRef<EmbeddedWorkflow, WorkflowParent>>),
}
impl ParentKind for WorkflowParent {}
```

`WorkflowParent` is a closed enum — only Workflow, ReusableWorkflow, and EmbeddedWorkflow are valid immediate parents of embedded entities. The `EmbeddedWorkflow` variant is boxed to break the recursive size cycle.

Embedding depth is represented by nesting `WorkflowParent` values, not by growing the static type. An `EntityRef<Task, WorkflowParent>` covers a task at any embedding depth — the concrete chain is resolved at runtime.

---

## Which Entities Use Which Parent

| Entity | `entity_ref` field type |
|---|---|
| Role, Hook, Team, ArtifactKind, Workflow, ReusableWorkflow | `EntityRef<T, NoParent>` |
| Task, Relay, EmbeddedWorkflow | `EntityRef<T, WorkflowParent>` |

The constraint that Relay cannot appear in `ReusableWorkflow` definitions is a domain constraint — not enforced by `ParentKind` at the type level.

---

## `NoParent` as ZST

`NoParent` is a zero-sized type. It contributes no bytes to `EntityRef`'s memory layout — the compiler elides it entirely.
