# substrate-trait

**Substrate Layer → `substrate_layer/substrate-trait/`**

---

## Purpose

`Substrate` is the trait that decouples the Store Layer from its backing storage. It has two roles:

1. **Component provider** — exposes the three substrate-specific components (Resolver, Codec, Executor) that the default pipeline implementations use.
2. **Operation interface** — exposes `persist`, `load`, and `exists` as the Store Layer-facing API, with default implementations provided by the framework using those three components.

`Store<S: Substrate>` is statically dispatched — the concrete substrate type is fixed at construction. No `dyn Substrate`.

---

## Definition

```rust
trait Substrate: Sized {
    // Substrate-specific types
    type Slot: Slot;
    type Location;
    type Encoded;
    type Resolver: LocationResolver<Location = Self::Location>;
    type Codec: Codec<Slot = Self::Slot, Encoded = Self::Encoded>;
    type Executor: Executor<Location = Self::Location, Encoded = Self::Encoded>;

    // Component accessors — the only required impl surface for a substrate author
    fn resolver(&self) -> &Self::Resolver;
    fn codec(&self) -> &Self::Codec;
    fn executor(&self) -> &Self::Executor;

    // Static query — no &self needed; used by EntityServer for prerequisite resolution
    // and ensure_mutable. Default impl dispatches via SubstrateSchema<Self> impls.
    fn load_strategy(entity_kind: EntityKind, field: &str) -> LoadStrategy
    where
        Role: SubstrateSchema<Self>,
        Hook: SubstrateSchema<Self>,
        Team: SubstrateSchema<Self>,
        ArtifactKind: SubstrateSchema<Self>,
        Workflow: SubstrateSchema<Self>,
        ReusableWorkflow: SubstrateSchema<Self>,
        EmbeddedWorkflow: SubstrateSchema<Self>,
        Task: SubstrateSchema<Self>,
        Relay: SubstrateSchema<Self>,
    {
        match entity_kind {
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

    // Store-facing operations — default implementations provided by the framework
    async fn persist(
        &self,
        changes: impl Iterator<Item = EntityChange<'_>>,
    ) -> Result<(), Vec<SubstrateError>> { ... }

    async fn load(
        &self,
        entity: &TrackedEntity,
        fields: &[&str],
    ) -> Result<TrackedEntity, SubstrateError> { ... }

    async fn exists(
        &self,
        refs: &[AnyEntityRef],
    ) -> Result<Vec<bool>, SubstrateError> { ... }
}
```

`EntitySchema` is substrate-layer internal — the Store layer never accesses it directly. `load_strategy` is the public boundary through which the EntityServer queries prerequisite and mutability information. See [54 · load-strategy](load-strategy.md).

---

## Layering

```
Store layer          EntityServer, Store, change tracking, checkout/commit
─────────────────────────────────────────────────────────────────────────
Substrate layer      Substrate trait + default implementations
  (generic)          AssetMapper, EntitySchema (type structure)
─────────────────────────────────────────────────────────────────────────
Substrate layer      Codec, Executor, LocationResolver, Slot,
  (impl-specific)    EntitySchema (values per entity type), AssetKind constants
```

The default implementations of `persist`, `load`, and `exists` use `self.resolver()`, `self.codec()`, and `self.executor()` directly — no Orchestrator struct is constructed. The orchestration logic lives in the trait's default methods.

---

## Default Implementations

**`persist`** — consumes the store's lazy `EntityChange` iterator; maps each change through AssetMapper → `self.resolver()` → `self.codec()`; accumulates `AssetRequest`s; executes atomically via `self.executor()`. See [74 · write-path](../pipeline/write-path.md).

**`load`** — selects assets covering the requested fields via AssetMapper; resolves paths via `self.resolver()`; executes GET requests via `self.executor()`; decodes via `self.codec()`; returns a partial `TrackedEntity`. See [73 · read-path](../pipeline/read-path.md).

**`exists`** — resolves the `ref_asset` path for each ref via `self.resolver()`; executes HEAD requests via `self.executor()`; returns `Vec<bool>`. See [73 · read-path](../pipeline/read-path.md).

---

## What a Substrate Implementor Provides

| | Role |
|---|---|
| `Slot` enum | Encoding targets for this substrate |
| `LocationResolver` impl | Path template → concrete location |
| `Codec` impl | Encode/decode between field values and substrate format |
| `Executor` impl | Execute batched `AssetRequest`s; owns atomicity |
| `SubstrateSchema<Self>` per entity type | Impl-specific declarative config: field→slot mappings and path templates |
| Component accessors + constructor | Wiring; startup work (e.g., stale dir cleanup) |

The default `persist`, `load`, and `exists` implementations are inherited — no orchestration code required.

---

## Static Dispatch

```rust
struct Store<S: Substrate> {
    substrate: S,
    // ...
}
```

The substrate type is fixed at construction. Generic methods on `Store<S>` call substrate methods without vtable overhead.
