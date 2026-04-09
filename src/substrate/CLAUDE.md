# src/substrate — Persistence Backend

## Substrate Trait

**`substrate::Substrate`** (`src/substrate/mod.rs`) — the single persistence backend trait, used by both the EntityServer and the pipeline:
```rust
pub trait Substrate: Sized + Send + Sync + 'static {
    type Slot: pipeline::Slot;
    type Location: Send;
    type Encoded: Send;
    type Resolver: pipeline::LocationResolver<Location = Self::Location>;
    type Codec:    pipeline::Codec<Slot = Self::Slot, Encoded = Self::Encoded>;
    type Executor: pipeline::Executor<Location = Self::Location, Encoded = Self::Encoded>;

    fn resolver(&self) -> &Self::Resolver;
    fn codec(&self) -> &Self::Codec;
    fn executor(&self) -> &Self::Executor;
    fn load_strategy(entity_kind: EntityKind, field: &str) -> pipeline::LoadStrategy;

    async fn exists(&self, refs: &[AnyEntityRef]) -> Result<Vec<bool>, SubstrateError>;
    async fn load(&self, entity: &TrackedEntity, fields: &[&str]) -> Result<TrackedEntity, SubstrateError>;
    async fn persist(&self, changes: impl Iterator<Item = EntityChange<'_>>) -> Result<(), Vec<SubstrateError>>;
}
```

`Store<S>` in the EntityServer is generic over `S: Substrate`. `InMemorySubstrate` implements `Substrate` with void/unit associated types, overriding `exists`, `load`, and `persist` directly.

---

## EntityChange (`src/substrate/mod.rs`)

```rust
pub enum EntityChange<'a> {
    Added(&'a StoreEntity),
    Modified(&'a StoreEntity, &'a [&'a str]),   // dirty field names
    Removed(&'a AnyEntityRef),
}
```

---

## SubstrateError (`src/substrate/error.rs`)

```rust
pub enum SubstrateError {
    Codec(CodecError),       // from CodecError
    Executor(ExecutorError), // from ExecutorError
}
```

---

## Pipeline Vocabulary (`src/substrate/pipeline/`)

Traits composing a complete persistence pipeline:

```
Slot           — marker trait for substrate-specific encoding targets (e.g. FrontmatterSlot, BodySlot)
LocationResolver — fn resolve(entity_id: &str, data: &Value) -> Location
Codec          — encode(fields, mappings) -> Result<Encoded, CodecError>
               — decode(encoded, mappings) -> Result<HashMap<&str, Value>, CodecError>
Executor       — put/post/patch/delete/get/head operations on Location
AssetMapper    — determines which assets to write for a given ChangeOp
```

**Primitive error types:**
```rust
// pipeline/codec/error.rs
CodecError { field: String, message: String }    // constructor: CodecError::new(field, msg)

// pipeline/executor/error.rs
ExecutorError { location: String, message: String }   // constructor: ExecutorError::new(loc, msg)
```

Both implement `ErrorCompose` + `OTelEmit`. Both carry `SpanTrace` + `Backtrace`.

---

## RepoSubstrate (`src/substrate/repo/`)

Filesystem-backed implementation. Uses atomic swap via `.part/`/`.old/` dirs.

```rust
pub struct RepoSubstrate {
    pub resolver: RepoLocationResolver,
    pub codec:    RepoCodec,
    pub executor: RepoExecutor,
}

RepoSubstrate::new(root: PathBuf) -> Result<Self, SubstrateError>
// Cleans up stale .part/ and .old/ dirs on construction
```

**Persistence strategy:**
1. Compute LCA of all changed file paths (`lca.rs`)
2. Stage changes in `<lca>.part/` — hard-link unchanged siblings
3. Atomic swap via `fs::rename`

**File layout on disk:**
```
roles/<id>.md
hooks/<id>.md
teams/<id>.md
artifact-kinds/<id>.md
workflows/<id>/README.md
reusable-workflows/<id>/README.md
```

**Markdown format:** YAML frontmatter (metadata fields) + Markdown body (`# <name>` heading for name field).

### Sub-modules

| File | Purpose |
|------|---------|
| `codec.rs` | `RepoCodec` — encode/decode YAML frontmatter + Markdown body |
| `executor.rs` | `RepoExecutor` — fs read/write via `RepoSlot` |
| `resolver.rs` | `RepoLocationResolver` — maps entity kind + id → file path |
| `slot.rs` | `RepoSlot` — frontmatter vs body slot markers |
| `schemas.rs` | Per-entity `SubstrateSchema` impls (field→slot mappings) |
| `render.rs` | Markdown+YAML renderers per entity type |
| `lca.rs` | LCA computation over file paths for atomic persistence scope |
| `storage.rs` | Legacy `RepoSubstrate` implementation (used by `storage_integration` tests) |

---

## VoidSubstrate

No-op substrate for tests that don't need persistence. Both `VoidSubstrate` and `InMemorySubstrate` implement `substrate::Substrate` — `VoidSubstrate` with void/unit associated types and no-op method bodies; `InMemorySubstrate` with void associated types overriding `exists`, `load`, and `persist` with in-memory implementations.
