# Task 10 — Substrate Trait and Pipeline

## Scope

Implement the generic substrate infrastructure:

1. `Substrate` trait — with associated types (`Slot`, `Location`, `Encoded`, `Resolver`, `Codec`, `Executor`) and default `persist`, `load`, `exists` implementations
2. `Slot` marker trait — substrate-specific encoding targets
3. `LocationResolver` trait — path template → concrete location
4. `Codec` trait — encode/decode between field values and substrate format
5. `Executor` trait — execute batched `AssetRequest`s atomically
6. `AssetRequest`, `AssetResponse`, `AssetOp` — per-asset operation vocabulary
7. `EntitySchema<S>`, `RefAssetDef<S>`, `AssetDef<S>`, `FieldMapping<S>` — declarative entity→asset mapping
8. `SubstrateSchema<Sub>` trait — implemented per (entity type, substrate) pair; provides `SCHEMA` const
9. `AssetMapper` — selects which assets to write given dirty fields
10. `LoadStrategy`, `LoadStrategyQuery` — derive load prerequisites from entity schema
11. `VoidSubstrate` — no-op substrate for tests that don't need persistence

---

## Files

- `src/substrate/mod.rs` — `Substrate` trait, `SubstrateError`, `LoadStrategy`, `VoidSubstrate`
- `src/substrate/pipeline/mod.rs` — `Slot`, `LocationResolver`, `Codec`, `Executor`, `EntitySchema`, `SubstrateSchema`, `AssetMapper`, op vocabulary
- `src/lib.rs` — `pub mod substrate;`

---

## Dependencies

- Task 03: `TrackedX` types and serde impls (Task 08)
- Task 04: `StoreEntity`, `AnyEntityRef`, `EntityKind`
- Task 09: `EntityChange` enum, `SubstrateError`

---

## Core Vocabulary (`src/substrate/pipeline/mod.rs`)

### `Slot` Marker Trait

```rust
pub trait Slot: Copy + 'static {}
```

### `AssetOp`

```rust
pub enum AssetOp<E> {
    Put(E),    // write (full overwrite)
    Post(E),   // create (some substrates distinguish create vs update)
    Patch(E),  // partial update (substrates with supports_partial: true)
    Delete,
    Get,
    Head,
}
```

### `AssetRequest` and `AssetResponse`

```rust
pub struct AssetRequest<L, E> {
    pub location: L,
    pub op: AssetOp<E>,
}

pub enum AssetResponse<E> {
    Done,
    Data(E),
    Exists(bool),
}
```

### `AssetKind`

```rust
pub struct AssetKind {
    pub distinguishes_create: bool,  // if true: Added → Post, Modified → Put
    pub supports_partial: bool,       // if true: Modified → Patch
}

pub const MARKDOWN_FILE: AssetKind = AssetKind { distinguishes_create: false, supports_partial: false };
pub const RAW_FILE: AssetKind      = AssetKind { distinguishes_create: false, supports_partial: false };
```

### `FieldMapping<S>`

```rust
pub struct FieldMapping<S: Slot> {
    pub key:  &'static str,
    pub slot: S,
}
```

### `RefAssetDef<S>` and `AssetDef<S>`

```rust
pub struct RefAssetDef<S: Slot> {
    pub path_template: &'static str,
    pub kind:          &'static AssetKind,
    pub fields:        &'static [FieldMapping<S>],
}

pub struct AssetDef<S: Slot> {
    pub path_template: &'static str,
    pub kind:          &'static AssetKind,
    pub fields:        &'static [FieldMapping<S>],
    /// Fields from the ref_asset whose values are needed to resolve this asset's path template.
    pub path_deps:     &'static [&'static str],
}
```

### `EntitySchema<S>`

```rust
pub struct EntitySchema<S: Slot> {
    pub ref_asset: RefAssetDef<S>,
    pub assets:    &'static [AssetDef<S>],
}

impl<S: Slot> EntitySchema<S> {
    /// Derive the LoadStrategy for a given field name.
    pub fn load_strategy_for(&self, field: &str) -> LoadStrategy {
        // Check ref_asset first
        if self.ref_asset.fields.iter().any(|f| f.key == field) {
            return LoadStrategy {
                prerequisites: self.ref_asset.path_deps(),
                mutable_without_load: false, // ref_asset is always multi-field
            };
        }
        // Check additional assets
        for asset in self.assets {
            if asset.fields.iter().any(|f| f.key == field) {
                return LoadStrategy {
                    prerequisites: asset.path_deps.to_vec(),
                    mutable_without_load: asset.kind.supports_partial || asset.fields.len() == 1,
                };
            }
        }
        // Default: field not in schema — no prerequisites, mutable without load
        LoadStrategy { prerequisites: vec![], mutable_without_load: true }
    }
}
```

`RefAssetDef` has no `path_deps` (its path depends only on `EntityRef`). Add a helper:
```rust
impl<S: Slot> RefAssetDef<S> {
    fn path_deps(&self) -> Vec<&'static str> { vec![] }
}
```

### `LoadStrategy`

```rust
pub struct LoadStrategy {
    pub prerequisites:       Vec<&'static str>,
    pub mutable_without_load: bool,
}
```

### `SubstrateSchema<Sub>` Trait

```rust
pub trait SubstrateSchema<Sub: Substrate>: Entity {
    const SCHEMA: EntitySchema<<Sub as Substrate>::Slot>;
}
```

### `AssetMapper`

```rust
pub struct AssetMapper;

impl AssetMapper {
    /// For Added entities: return all assets.
    /// For Modified entities with dirty_fields: return only assets containing ≥1 dirty field.
    pub fn select_for_write<'a, S: Slot>(
        schema: &'a EntitySchema<S>,
        dirty_fields: Option<&'a [&'a str]>,
    ) -> Vec<&'a dyn AssetLike<S>> {
        // Returns ref_asset always for Added (dirty_fields=None).
        // For Modified, filters by dirty field membership.
        todo!()
    }
}
```

### `LocationResolver` Trait

```rust
pub trait LocationResolver {
    type Location;
    fn resolve(&self, path_template: &str, entity_json: &serde_json::Value) -> Self::Location;
}
```

The template uses `{id}` for the entity id and `{parent.base}` for the parent workflow directory (used by Task and Relay).

### `Codec` Trait

```rust
use std::collections::HashMap;

pub trait Codec {
    type Slot: Slot;
    type Encoded;

    fn encode(
        &self,
        fields: &HashMap<&str, serde_json::Value>,
        schema: &[FieldMapping<Self::Slot>],
    ) -> Result<Self::Encoded, CodecError>;

    fn decode(
        &self,
        raw: &Self::Encoded,
        schema: &[FieldMapping<Self::Slot>],
    ) -> Result<HashMap<String, serde_json::Value>, CodecError>;
}

pub struct CodecError { pub field: String, pub message: String }
```

### `Executor` Trait

```rust
pub trait Executor {
    type Location;
    type Encoded;

    fn execute(
        &self,
        ops: Vec<AssetRequest<Self::Location, Self::Encoded>>,
    ) -> Result<Vec<AssetResponse<Self::Encoded>>, Vec<ExecutorError>>;
}

pub struct ExecutorError { pub location: String, pub message: String }
```

---

## `Substrate` Trait (`src/substrate/mod.rs`)

```rust
pub trait Substrate: Sized + Send + Sync + 'static {
    type Slot: Slot;
    type Location: Send;
    type Encoded: Send;
    type Resolver: LocationResolver<Location = Self::Location>;
    type Codec: Codec<Slot = Self::Slot, Encoded = Self::Encoded>;
    type Executor: Executor<Location = Self::Location, Encoded = Self::Encoded>;

    fn resolver(&self) -> &Self::Resolver;
    fn codec(&self)    -> &Self::Codec;
    fn executor(&self) -> &Self::Executor;

    /// Derive LoadStrategy for a (entity_kind, field) pair.
    fn load_strategy(entity_kind: EntityKind, field: &str) -> LoadStrategy;

    /// Check existence of a single entity.
    async fn exists(&self, any_ref: &AnyEntityRef) -> Result<bool, SubstrateError> {
        // Default: resolve ref_asset path, issue Head request, check Exists(true)
        todo!("default impl: resolver + executor Head")
    }

    /// Load the specified fields of an entity. `fields: &[]` = all fields.
    async fn load(
        &self,
        any_ref: &AnyEntityRef,
        fields: &[&str],
    ) -> Result<StoreEntity, SubstrateError> {
        // Default: AssetMapper selects assets, resolver + executor Get, codec decode, serde_json::from_value
        todo!("default impl: resolver + executor Get + codec decode")
    }

    /// Persist a set of entity changes atomically.
    async fn atomic_persist(
        &self,
        changes: &[EntityChange<'_>],
    ) -> Result<(), Vec<SubstrateError>> {
        // Default impl: AssetMapper + resolver + codec encode + executor execute
        todo!("default impl: AssetMapper + resolver + codec + executor")
    }
}
```

The default implementations are non-trivial and are part of this task's implementation work. They replace the previous `atomic_persist` signature.

---

## `VoidSubstrate` (for tests without persistence)

```rust
pub struct VoidSubstrate;

#[derive(Clone, Copy)]
pub struct VoidSlot;
impl Slot for VoidSlot {}

impl Substrate for VoidSubstrate {
    type Slot     = VoidSlot;
    type Location = String;
    type Encoded  = String;
    // Resolver, Codec, Executor are unit types that panic on use
    // ...

    fn load_strategy(_: EntityKind, _: &str) -> LoadStrategy {
        LoadStrategy { prerequisites: vec![], mutable_without_load: true }
    }

    async fn exists(&self, _: &AnyEntityRef) -> Result<bool, SubstrateError> { Ok(false) }
    async fn load(&self, any_ref: &AnyEntityRef, _: &[&str]) -> Result<StoreEntity, SubstrateError> {
        Err(SubstrateError { path: any_ref.id().to_string(), message: "VoidSubstrate: no load".to_string() })
    }
    async fn atomic_persist(&self, _: &[EntityChange<'_>]) -> Result<(), Vec<SubstrateError>> { Ok(()) }
}
```

---

## TDD: Tests to Write First

```rust
// tests/substrate_pipeline.rs
use pari::substrate::{VoidSubstrate, Substrate};
use pari::entity::{AnyEntityRef, EntityRef, EntityKind};
use pari::entities::role::Role;

#[tokio::test]
async fn void_substrate_exists_returns_false() {
    let sub = VoidSubstrate;
    let r: EntityRef<Role> = EntityRef::new("eng-lead");
    let any = AnyEntityRef::Role(r);
    assert!(!sub.exists(&any).await.unwrap());
}

#[tokio::test]
async fn void_substrate_persist_succeeds() {
    let sub = VoidSubstrate;
    sub.atomic_persist(&[]).await.unwrap();
}

// EntitySchema LoadStrategy derivation

use pari::substrate::pipeline::{EntitySchema, RefAssetDef, AssetDef, FieldMapping, MARKDOWN_FILE, RAW_FILE};

// Use a test slot enum
#[derive(Clone, Copy)]
enum TestSlot { H1, FrontmatterKey(&'static str), FileContent }
impl pari::substrate::pipeline::Slot for TestSlot {}

#[test]
fn load_strategy_for_ref_asset_field() {
    let schema: EntitySchema<TestSlot> = EntitySchema {
        ref_asset: RefAssetDef {
            path_template: "roles/{id}.md",
            kind: &MARKDOWN_FILE,
            fields: &[
                FieldMapping { key: "name",    slot: TestSlot::H1 },
                FieldMapping { key: "purpose", slot: TestSlot::FrontmatterKey("purpose") },
            ],
        },
        assets: &[],
    };
    let strategy = schema.load_strategy_for("name");
    assert!(strategy.prerequisites.is_empty());
    assert!(!strategy.mutable_without_load); // ref_asset = multi-field = must load first
}

#[test]
fn load_strategy_for_single_field_asset() {
    let schema: EntitySchema<TestSlot> = EntitySchema {
        ref_asset: RefAssetDef {
            path_template: "tasks/{id}/README.md",
            kind: &MARKDOWN_FILE,
            fields: &[FieldMapping { key: "name", slot: TestSlot::H1 }],
        },
        assets: &[AssetDef {
            path_template: "tasks/{id}/template.md",
            kind: &RAW_FILE,
            fields: &[FieldMapping { key: "template_content", slot: TestSlot::FileContent }],
            path_deps: &[],
        }],
    };
    let strategy = schema.load_strategy_for("template_content");
    assert!(strategy.mutable_without_load, "single-field asset should be mutable without load");
}

#[test]
fn load_strategy_for_unknown_field_is_mutable_without_load() {
    let schema: EntitySchema<TestSlot> = EntitySchema {
        ref_asset: RefAssetDef {
            path_template: "x/{id}.md",
            kind: &MARKDOWN_FILE,
            fields: &[],
        },
        assets: &[],
    };
    let strategy = schema.load_strategy_for("nonexistent_field");
    assert!(strategy.mutable_without_load);
    assert!(strategy.prerequisites.is_empty());
}
```

---

## Implementation Notes

### Default `persist` Implementation

The default `atomic_persist` implementation:
1. For each `EntityChange`:
   - `Added(entity)` or `Modified(entity, dirty_fields)`: call `serde_json::to_value(entity)` to get field values; use `AssetMapper::select_for_write` to select assets; for each selected asset: resolve path via `self.resolver()`, extract field values, encode via `self.codec()`, build `AssetRequest { op: Put(...) }`
   - `Removed(any_ref)`: resolve ref_asset path via `self.resolver()`, build `AssetRequest { op: Delete }`
2. Call `self.executor().execute(ops)` — atomicity is the executor's responsibility
3. Map executor errors to `Vec<SubstrateError>`

### Default `load` Implementation

The default `load` implementation:
1. Determine which assets contain the requested fields (or all assets if `fields: &[]`)
2. For each asset: resolve path, build `AssetRequest { op: Get }`, execute
3. For each response: decode via `self.codec()` using the asset's `FieldMapping` slice
4. Merge decoded field name → JSON value maps into a single map
5. Call `serde_json::from_value(map)` to get a partial `StoreEntity` (only present keys initialized)

### `load_strategy` on `Substrate` Trait

The `load_strategy` static method must be implemented per substrate. For substrates implementing `SubstrateSchema<Self>` for all entity types, a default dispatch via `EntityKind` match is generated (similar to the `entity_registry!` approach). Task 11 implements `load_strategy` for `RepoSubstrate`.

---

## Acceptance Criteria

- `cargo test substrate_pipeline` passes
- `VoidSubstrate::exists` returns `false`; `atomic_persist` returns `Ok(())`; `load` returns `Err`
- `EntitySchema::load_strategy_for` correctly derives prerequisites and mutable_without_load
- `EntitySchema::load_strategy_for` for a single-field additional asset returns `mutable_without_load: true`
- `Substrate` trait compiles with all associated types and default methods
- Tasks 01-09 tests still pass
