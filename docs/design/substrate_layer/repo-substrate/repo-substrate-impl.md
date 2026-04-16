# repo-substrate-impl

**Owning layer: `substrate`**

---

## Purpose

`RepoSubstrate` wires the three component accessors (`resolver`, `codec`, `executor`) into the `Substrate` trait. All operations — `load_strategy`, `persist`, `load`, `exists` — are fully inherited from the `Substrate` trait's default implementations. `RepoSubstrate` also handles startup cleanup of stale staging directories.

---

## Structure

```rust
struct RepoSubstrate {
    resolver: RepoLocationResolver,
    codec:    RepoCodec,
    executor: RepoExecutor,
}

impl RepoSubstrate {
    pub fn new(root: PathBuf) -> Result<Self, SubstrateError> {
        Self::cleanup_stale(&root)?;
        Ok(Self {
            resolver: RepoLocationResolver::new(root.clone()),
            codec:    RepoCodec,
            executor: RepoExecutor::new(root),
        })
    }

    fn cleanup_stale(root: &Path) -> Result<(), SubstrateError> {
        // scan for *.part/ and *.old/ directories, remove them
    }
}
```

---

## Substrate Trait Impl

`RepoSubstrate` implements only the three required component accessors. All other operations — `load_strategy`, `persist`, `load`, `exists` — are fully inherited from the `Substrate` trait's default implementations. `load_strategy` dispatches via the `SubstrateSchema<RepoSubstrate>` impls provided in `schema.rs`.

```rust
impl Substrate for RepoSubstrate {
    type Slot     = RepoSlot;
    type Location = PathBuf;
    type Encoded  = String;
    type Resolver = RepoLocationResolver;
    type Codec    = RepoCodec;
    type Executor = RepoExecutor;

    fn resolver(&self) -> &Self::Resolver { &self.resolver }
    fn codec(&self)    -> &Self::Codec    { &self.codec }
    fn executor(&self) -> &Self::Executor { &self.executor }
}
```

`persist`, `load`, and `exists` are not implemented here — they are fully derived from the default implementations on the `Substrate` trait via `self.resolver()`, `self.codec()`, and `self.executor()`.

---

## Schema Dispatch

`RepoSubstrate` has no `load_strategy` override. Entity-type knowledge lives entirely in the `SubstrateSchema<RepoSubstrate>` impls in `schema.rs` — one per entity type, each a `const EntitySchema<RepoSlot>` declaring field→slot mappings, path templates, and `path_deps`. The `Substrate` trait's default `load_strategy` dispatches through these impls. Everything else — asset selection, path resolution, encoding, execution — is driven by the schema values and the generic default implementations.

---

## Startup Cleanup

On `new()`, before accepting any operations, `RepoSubstrate` scans the repository root for stale `.part/` and `.old/` directories left by a crashed previous process. These are removed unconditionally. A `.part/` directory represents an incomplete staged write — its contents are not valid repository state and must be discarded.
