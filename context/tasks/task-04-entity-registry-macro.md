# Task 04 — `entity_registry!` Macro

## Scope

Implement the `entity_registry!` declarative macro in `pari-macros/src/lib.rs`. Given a list of entity type names, the macro generates:

1. `EntityKind` enum — unit enum with one variant per entity type; replaces the hand-written stub from Task 02
2. `AnyEntityRef` enum — one variant per entity type wrapping `EntityRef<T, T::Parent>`
3. `StoreEntity` enum — one variant per entity type wrapping the `TrackedX` struct (this is the store-level enum; the companion trait is named `TrackedEntity` — these are distinct)
4. `AnyEntityRef::kind()`, `AnyEntityRef::id()`, `AnyEntityRef::parent()` accessor methods
5. `load_strategy()` function — dispatches to per-entity `SubstrateSchema` implementations

This task also replaces the hand-written `EntityKind`, `AnyEntityRef`, and `StoreEntity` stubs from Task 02.

---

## Files

- `pari-macros/src/lib.rs` — add `entity_registry!` proc macro (or declarative macro via `macro_rules!`)
- `src/entity.rs` — remove hand-written `EntityKind` stub; remove `AnyEntityRef` and `StoreEntity` stubs; add `entity_registry!` invocation at bottom of file
- `src/lib.rs` — ensure macro is exported

---

## Dependencies

- Task 02: `Entity`, `TrackedEntity` (companion trait), `EntityRef`, `NoParent`, `WorkflowParent`, `ParentKind`
- Task 03: `TrackedX` structs (generated for each entity type) — only needed for `StoreEntity` variants; stubs suffice for Tasks 04's own tests
- Task 05: Plain entity structs (Role, Hook, etc.) — only needed for the final registry invocation

---

## Macro Invocation (placed in `src/entity.rs` or `src/lib.rs`)

```rust
entity_registry! {
    Role       => NoParent,
    Hook       => NoParent,
    Team       => NoParent,
    Workflow   => NoParent,
    ReusableWorkflow => NoParent,
    ArtifactKind     => NoParent,
    Task             => WorkflowParent,
    Relay            => WorkflowParent,
    EmbeddedWorkflow => WorkflowParent,
}
```

Each entry is `TypeName => ParentKindType`. The macro does not read `T::Parent` at macro expansion time (proc macros can't resolve trait associated types); the parent type is provided explicitly in the invocation.

---

## Generated: `EntityKind`

Replaces the hand-written stub. Identical content:

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

impl AnyEntityRef {
    /// Returns the EntityKind discriminant for this ref.
    pub fn kind(&self) -> EntityKind {
        match self {
            AnyEntityRef::Role(_)             => EntityKind::Role,
            AnyEntityRef::Hook(_)             => EntityKind::Hook,
            AnyEntityRef::Team(_)             => EntityKind::Team,
            AnyEntityRef::Workflow(_)         => EntityKind::Workflow,
            AnyEntityRef::ReusableWorkflow(_) => EntityKind::ReusableWorkflow,
            AnyEntityRef::ArtifactKind(_)     => EntityKind::ArtifactKind,
            AnyEntityRef::Task(_)             => EntityKind::Task,
            AnyEntityRef::Relay(_)            => EntityKind::Relay,
            AnyEntityRef::EmbeddedWorkflow(_) => EntityKind::EmbeddedWorkflow,
        }
    }

    /// Returns the id string of the entity.
    pub fn id(&self) -> &str {
        match self {
            AnyEntityRef::Role(r)             => r.id(),
            AnyEntityRef::Hook(r)             => r.id(),
            AnyEntityRef::Team(r)             => r.id(),
            AnyEntityRef::Workflow(r)         => r.id(),
            AnyEntityRef::ReusableWorkflow(r) => r.id(),
            AnyEntityRef::ArtifactKind(r)     => r.id(),
            AnyEntityRef::Task(r)             => r.id(),
            AnyEntityRef::Relay(r)            => r.id(),
            AnyEntityRef::EmbeddedWorkflow(r) => r.id(),
        }
    }

    /// Returns the parent AnyEntityRef for embedded entities, None for top-level.
    pub fn parent(&self) -> Option<AnyEntityRef> {
        match self {
            AnyEntityRef::Task(r) =>
                Some(AnyEntityRef::Workflow(EntityRef::new(r.parent.workflow_id.clone()))),
            AnyEntityRef::Relay(r) =>
                Some(AnyEntityRef::Workflow(EntityRef::new(r.parent.workflow_id.clone()))),
            AnyEntityRef::EmbeddedWorkflow(r) =>
                Some(AnyEntityRef::Workflow(EntityRef::new(r.parent.workflow_id.clone()))),
            _ => None,
        }
    }
}
```

**Note on `parent()`**: Embedded entities' parent is always a `Workflow` (by convention in this design). The parent is reconstructed from the `WorkflowParent.workflow_id` field. This is a deliberate simplification — `EmbeddedWorkflow` and `Relay` cannot live inside `ReusableWorkflow` per the step type hierarchy.

---

## Generated: `StoreEntity`

The store-level enum. One variant per entity type, holding the `TrackedX` struct. **Not to be confused with the `TrackedEntity` companion trait** (defined in Task 02).

```rust
pub enum StoreEntity {
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

---

## Generated: `load_strategy`

Dispatches to per-entity substrate codecs. Uses a `SubstrateSchema` trait (stub for this task; filled in by Task 12):

```rust
// Stub trait — Task 12 provides the real definition
pub trait SubstrateSchema: Send + Sync {
    fn kind(&self) -> EntityKind;
}

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

The `match` has no wildcard arm — any missing entity type is a compile error.

During this task, each `XxxSchema` is a zero-sized stub that implements `SubstrateSchema`:

```rust
struct RoleSchema;
impl SubstrateSchema for RoleSchema {
    fn kind(&self) -> EntityKind { EntityKind::Role }
}
// ... repeat for all types
```

---

## TDD: Tests to Write First

```rust
#[cfg(test)]
mod tests {
    use super::*;

    // --- EntityKind ---

    #[test]
    fn entity_kind_all_variants_distinct() {
        let kinds = [
            EntityKind::Role, EntityKind::Hook, EntityKind::Team,
            EntityKind::Workflow, EntityKind::ReusableWorkflow, EntityKind::ArtifactKind,
            EntityKind::Task, EntityKind::Relay, EntityKind::EmbeddedWorkflow,
        ];
        // All variants are distinct
        for (i, a) in kinds.iter().enumerate() {
            for (j, b) in kinds.iter().enumerate() {
                if i != j { assert_ne!(a, b); }
            }
        }
    }

    #[test]
    fn entity_kind_is_copy() {
        let k = EntityKind::Role;
        let k2 = k; // copy, no move
        assert_eq!(k, k2);
    }

    // --- AnyEntityRef construction ---

    #[test]
    fn any_entity_ref_role_kind() {
        // Need a Role type; use a stub if real Role not defined yet
        // This test requires that Entity impls exist for Role — run after Task 05
        // For Task 04 tests, use a minimal stub entity and manual variant construction
    }

    #[test]
    fn any_entity_ref_id_accessor() {
        // Can be tested without full entity types by constructing AnyEntityRef directly
        // Requires EntityRef to be constructable — available from Task 02
        // Use a concrete entity type from Task 05 once available
        // For now: verify the enum compiles and kind() / id() methods exist
        let _ = AnyEntityRef::kind;
        let _ = AnyEntityRef::id;
    }

    // --- AnyEntityRef::kind ---

    #[test]
    fn any_entity_ref_kind_matches_variant() {
        // After Task 05 entities exist:
        // let r = AnyEntityRef::Role(EntityRef::new("r"));
        // assert_eq!(r.kind(), EntityKind::Role);
        //
        // For Task 04: verify EntityKind variants are generated correctly (covered above)
    }

    // --- AnyEntityRef::parent ---

    #[test]
    fn top_level_entity_ref_has_no_parent() {
        // After Task 05:
        // let r = AnyEntityRef::Role(EntityRef::new("r"));
        // assert!(r.parent().is_none());
    }

    #[test]
    fn embedded_entity_ref_has_workflow_parent() {
        // After Task 05:
        // let r = AnyEntityRef::Task(EntityRef::new_embedded("WriteProposal", "InitiativeWorkflow"));
        // let parent = r.parent().unwrap();
        // assert_eq!(parent.id(), "InitiativeWorkflow");
        // assert_eq!(parent.kind(), EntityKind::Workflow);
    }

    // --- load_strategy ---

    #[test]
    fn load_strategy_returns_correct_kind_for_each_entity() {
        assert_eq!(load_strategy(EntityKind::Role).kind(), EntityKind::Role);
        assert_eq!(load_strategy(EntityKind::Hook).kind(), EntityKind::Hook);
        assert_eq!(load_strategy(EntityKind::Team).kind(), EntityKind::Team);
        assert_eq!(load_strategy(EntityKind::Workflow).kind(), EntityKind::Workflow);
        assert_eq!(load_strategy(EntityKind::ReusableWorkflow).kind(), EntityKind::ReusableWorkflow);
        assert_eq!(load_strategy(EntityKind::ArtifactKind).kind(), EntityKind::ArtifactKind);
        assert_eq!(load_strategy(EntityKind::Task).kind(), EntityKind::Task);
        assert_eq!(load_strategy(EntityKind::Relay).kind(), EntityKind::Relay);
        assert_eq!(load_strategy(EntityKind::EmbeddedWorkflow).kind(), EntityKind::EmbeddedWorkflow);
    }

    // --- StoreEntity ---

    #[test]
    fn store_entity_and_any_entity_ref_have_same_variant_names() {
        // This is a compile-time guarantee from the macro — if variants diverge,
        // Resolvable::extract impls (generated in Task 03) will fail to compile.
        // Verified by the fact that the crate compiles.
    }
}
```

---

## Implementation Notes

### Macro Strategy: `macro_rules!` vs Proc Macro

`entity_registry!` can be implemented as either a `macro_rules!` declarative macro or as a `proc_macro!` in `pari-macros`. Recommended: proc macro (`proc_macro!` attribute), since it needs to generate top-level items (`enum`, `impl`, `fn`) that declarative macros can produce but are awkward for large outputs.

Alternatively, `macro_rules!` with `$( $name:ident => $parent:ty ),+` pattern is simpler to write for this case. Use whichever approach compiles cleanly.

### Replacing Task 02 Stubs

Task 02's `entity.rs` contains hand-written `EntityKind`, `AnyEntityRef`, and `StoreEntity` (empty enums with `// Filled in by entity_registry! in Task 04` comments). This task removes those stubs and generates the real definitions via `entity_registry!`. Tests from Task 02 that reference `EntityKind::Role`, `EntityKind::Task` etc. continue to work because the real `EntityKind` has the same variants.

### `AnyEntityRef::parent()` — Workflow-only assumption

The `parent()` accessor hard-codes the parent variant as `AnyEntityRef::Workflow`. This is correct for the current entity hierarchy where all embedded entities live inside workflows. If the hierarchy changes, `parent()` needs updating — this assumption is intentional and matches the step type design.

### `StoreEntity` naming

The store-level enum is named `StoreEntity` throughout (not `TrackedEntity`) to avoid the naming collision with the `TrackedEntity` companion trait from Task 02. Every file and comment uses `StoreEntity` for the enum; `TrackedEntity` exclusively refers to the companion trait.

---

## Acceptance Criteria

- `cargo test entity_registry` passes
- `EntityKind` has exactly 9 variants (Role, Hook, Team, Workflow, ReusableWorkflow, ArtifactKind, Task, Relay, EmbeddedWorkflow); all Copy + Clone + PartialEq + Eq + Hash
- `AnyEntityRef` has exactly 9 variants; `kind()`, `id()`, `parent()` methods present
- `StoreEntity` has exactly 9 variants
- `load_strategy` is exhaustive — adding a 10th entity type to the registry without a `SubstrateSchema` impl causes a compile error
- Task 02 tests (`cargo test entity_identity`) still pass after stubs are replaced
