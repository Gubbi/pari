# substrate-schema-trait

**Owning layer: `substrate`**

---

## Purpose

`SubstrateSchema<Sub: Substrate>` associates a static `EntitySchema` with each entity type for a given substrate. Parameterising over the substrate (not just its slot type) makes the coupling explicit — an impl directly declares "this is how entity type X maps to substrate Y".

Completeness is enforced at compile time — a missing impl is a compile error, not a runtime panic.

---

## Trait

```rust
trait SubstrateSchema<Sub: Substrate> {
    const SCHEMA: EntitySchema<Sub::Slot>;
}
```

Implemented once per (entity type, substrate) pair:

```rust
impl SubstrateSchema<RepoSubstrate> for Role { const SCHEMA: EntitySchema<RepoSlot> = ...; }
impl SubstrateSchema<RepoSubstrate> for Hook { const SCHEMA: EntitySchema<RepoSlot> = ...; }
impl SubstrateSchema<RepoSubstrate> for Team { const SCHEMA: EntitySchema<RepoSlot> = ...; }
// ... all entity types
```

The slot type (`RepoSlot`) is an implementation detail — it appears in the const type but is not the key. The key is the substrate type (`RepoSubstrate`).

---

## Where Impls Live

Impls live in the substrate module (e.g., `substrate/repo/schema.rs`) — not in the entity module. This avoids orphan rule violations: the substrate crate owns the substrate type and can provide impls for foreign entity types.

---

## Compile-Time Completeness

The `Substrate` trait's default `load_strategy`, `persist`, `load`, and `exists` methods are generic over `SubstrateSchema<Self>`:

```rust
fn load_strategy(kind: EntityKind, field: &str) -> LoadStrategy
where
    Role:             SubstrateSchema<Self>,
    Hook:             SubstrateSchema<Self>,
    Team:             SubstrateSchema<Self>,
    ArtifactKind:     SubstrateSchema<Self>,
    Workflow:         SubstrateSchema<Self>,
    ReusableWorkflow: SubstrateSchema<Self>,
    EmbeddedWorkflow: SubstrateSchema<Self>,
    Task:             SubstrateSchema<Self>,
    Relay:            SubstrateSchema<Self>,
{
    match kind {
        EntityKind::Role             => <Role             as SubstrateSchema<Self>>::SCHEMA.load_strategy_for(field),
        EntityKind::Hook             => <Hook             as SubstrateSchema<Self>>::SCHEMA.load_strategy_for(field),
        EntityKind::Team             => <Team             as SubstrateSchema<Self>>::SCHEMA.load_strategy_for(field),
        EntityKind::ArtifactKind     => <ArtifactKind     as SubstrateSchema<Self>>::SCHEMA.load_strategy_for(field),
        EntityKind::Workflow         => <Workflow         as SubstrateSchema<Self>>::SCHEMA.load_strategy_for(field),
        EntityKind::ReusableWorkflow => <ReusableWorkflow as SubstrateSchema<Self>>::SCHEMA.load_strategy_for(field),
        EntityKind::EmbeddedWorkflow => <EmbeddedWorkflow as SubstrateSchema<Self>>::SCHEMA.load_strategy_for(field),
        EntityKind::Task             => <Task             as SubstrateSchema<Self>>::SCHEMA.load_strategy_for(field),
        EntityKind::Relay            => <Relay            as SubstrateSchema<Self>>::SCHEMA.load_strategy_for(field),
    }
}
```

If an entity type is added without a corresponding `SubstrateSchema<RepoSubstrate>` impl, the code fails to compile. No runtime "schema not found" errors.

---

## LoadStrategy Derivation

`EntitySchema::load_strategy_for` is the method that drives the derivation:

```rust
impl<S: Slot> EntitySchema<S> {
    fn load_strategy_for(&self, field: &str) -> LoadStrategy {
        if self.ref_asset.fields.iter().any(|f| f.key == field) {
            return LoadStrategy {
                prerequisites: &[],
                mutable_without_load: self.ref_asset.kind.supports_partial
                    || self.ref_asset.fields.len() == 1,
            };
        }
        for asset in self.assets {
            if asset.fields.iter().any(|f| f.key == field) {
                return LoadStrategy {
                    prerequisites: asset.path_deps,
                    mutable_without_load: asset.kind.supports_partial
                        || asset.fields.len() == 1,
                };
            }
        }
        // field not in schema — unreachable in well-formed code
        LoadStrategy { prerequisites: &[], mutable_without_load: true }
    }
}
```

Because `load_strategy` is now fully defaulted on the `Substrate` trait via `SubstrateSchema<Self>`, substrate implementations (e.g., `RepoSubstrate`) do not need to override it.
