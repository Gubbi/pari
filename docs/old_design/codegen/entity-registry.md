# entity-registry

**Generated behavior layers: `entity`, `store`, `substrate`**

---

## Purpose

`entity_registry!` is a declarative macro invoked once in the crate. It takes the full list of entity types and generates aggregate types whose behavior belongs to the formal `entity`, `store`, and `substrate` layers. This doc lives under `codegen/` because the mechanism is a macro, but the generated outputs are not a separate architectural layer. Adding a new entity type requires only adding it to this list — the generated types stay in sync at compile time.

---

## Invocation

Each entry declares the entity type and its parent kind explicitly:

```rust
entity_registry! {
    Role             => NoParent,
    Hook             => NoParent,
    Team             => NoParent,
    Workflow         => NoParent,
    ReusableWorkflow => NoParent,
    ArtifactKind     => NoParent,
    Task             => WorkflowParent,
    Relay            => WorkflowParent,
    EmbeddedWorkflow => WorkflowParent,
}
```

---

## Generated: `EntityKind`

A unit enum with one variant per registered entity type. Used as a runtime discriminant in `EntityRef`, `AnyEntityRef` dispatch, and substrate `load_strategy`.

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EntityKind {
    Role,
    Hook,
    Team,
    Workflow,
    ReusableWorkflow,
    ArtifactKind,
    Task,
    Relay,
    EmbeddedWorkflow,
}
```

---

## Generated: `AnyEntityRef`

One variant per entity type, each wrapping the concrete `EntityRef<T, T::Parent>`. Used as the key type in the store's `HashMap`.

```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum AnyEntityRef {
    Role(EntityRef<Role, NoParent>),
    Hook(EntityRef<Hook, NoParent>),
    Team(EntityRef<Team, NoParent>),
    Workflow(EntityRef<Workflow, NoParent>),
    ReusableWorkflow(EntityRef<ReusableWorkflow, NoParent>),
    ArtifactKind(EntityRef<ArtifactKind, NoParent>),
    Task(EntityRef<Task, WorkflowParent>),
    Relay(EntityRef<Relay, WorkflowParent>),
    EmbeddedWorkflow(EntityRef<EmbeddedWorkflow, WorkflowParent>),
}
```

`Parent` is taken from the `=> ParentType` declaration in the registry invocation.

---

## Generated: `TrackedEntity` enum

One variant per entity type, each wrapping the concrete tracked struct. Used as the value type in the store's `HashMap`.

```rust
pub enum TrackedEntity {
    Role(TrackedRole),
    Hook(TrackedHook),
    Team(TrackedTeam),
    Workflow(TrackedWorkflow),
    ReusableWorkflow(TrackedReusableWorkflow),
    ArtifactKind(TrackedArtifactKind),
    Task(TrackedTask),
    Relay(TrackedRelay),
    EmbeddedWorkflow(TrackedEmbeddedWorkflow),
}
```

`AnyEntityRef` and `TrackedEntity` grow together — the macro guarantees they have the same set of variants.

---

## Generated: `load_strategy`

A dispatch function used by the substrate to route a load request to the correct codec:

```rust
pub fn load_strategy(kind: EntityKind) -> &'static dyn SubstrateSchema {
    match kind {
        EntityKind::Role             => &RoleSchema,
        EntityKind::Hook             => &HookSchema,
        EntityKind::Team             => &TeamSchema,
        EntityKind::Workflow         => &WorkflowSchema,
        EntityKind::ReusableWorkflow => &ReusableWorkflowSchema,
        EntityKind::ArtifactKind     => &ArtifactKindSchema,
        EntityKind::Task             => &TaskSchema,
        EntityKind::Relay            => &RelaySchema,
        EntityKind::EmbeddedWorkflow => &EmbeddedWorkflowSchema,
    }
}
```

The `match` is exhaustive — omitting a variant is a compile error. Each arm returns a `&'static dyn SubstrateSchema` whose impl contains the file-path template, serializer, and deserializer for that entity type. If a `SubstrateSchema` impl is missing for any registered entity type, the crate fails to compile.

---

## Compile-Time Completeness

The macro generates a `match` with no wildcard arm for `load_strategy` and derives no default for `EntityKind`. Any entity type added to the registry without a corresponding `SubstrateSchema` impl produces a compile error at the `load_strategy` match, not a runtime panic.

---

## Summary

| Generated artifact | Purpose |
|---|---|
| `EntityKind` | runtime discriminant for refs and dispatch |
| `AnyEntityRef` | type-erased store key |
| `TrackedEntity` | type-erased store value |
| `load_strategy` | substrate codec dispatch with compile-time exhaustiveness |
