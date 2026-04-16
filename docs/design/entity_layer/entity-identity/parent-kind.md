# parent-kind

**Owning layer: `entity`**

---

## Purpose

`EntityRef<T, P>` carries a parent type parameter `P`. `ParentKind` is the trait bounding `P`, constraining which types are valid parents at compile time. This is the authoritative mechanism for expressing entity hierarchy in the model.

---

## `ParentKind` Trait and Implementors

```rust
pub trait ParentKind {}

/// For top-level entities with no parent.
pub struct NoParent;
impl ParentKind for NoParent {}

/// Concrete parent type for embedded entities in the workflow hierarchy.
pub enum WorkflowParent {
    Workflow(EntityRef<Workflow>),
    ReusableWorkflow(EntityRef<ReusableWorkflow>),
    EmbeddedWorkflow(Box<EntityRef<EmbeddedWorkflow, WorkflowParent>>),
}
impl ParentKind for WorkflowParent {}
```

`WorkflowParent` is a closed enum — only `Workflow`, `ReusableWorkflow`, and `EmbeddedWorkflow` are valid immediate parents of embedded entities in this hierarchy. The `EmbeddedWorkflow` variant is boxed to break the recursive size cycle.

Hierarchy depth is represented by nesting `WorkflowParent` values, not by growing the static type. An `EntityRef<Task, WorkflowParent>` covers a task at any depth under a workflow tree — the concrete chain is resolved at runtime.

This hierarchy is about identity and relationship, not about how the Store lays entities out internally. The Store keeps a flat map keyed by full `AnyEntityRef`; the parent chain is part of the key and part of the meaning of the entity.

---

## Which Entities Use Which Parent

| Entity | `entity_ref` field type |
|---|---|
| Role, Hook, Team, ArtifactKind, Workflow, ReusableWorkflow | `EntityRef<T, NoParent>` |
| Task, Relay, EmbeddedWorkflow | `EntityRef<T, WorkflowParent>` |

The constraint that `Relay` cannot appear in a `ReusableWorkflow` tree is a domain constraint — not enforced by `ParentKind` at the type level.

---

## `NoParent` as ZST

`NoParent` is a zero-sized type. It contributes no bytes to `EntityRef`'s memory layout — the compiler elides it entirely.
