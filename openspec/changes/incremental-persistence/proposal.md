## Why

The substrate layer currently persists the entire EntityStore on every call — even when a single field on a single entity changes. This is wasteful for local filesystem persistence and prohibitive for future substrate implementations (remote databases, Notion, APIs) where write operations are expensive. Fine-grained change tracking enables incremental persistence: only modified entities are re-rendered and written.

## What Changes

- Introduce `Tracked<T>` newtype with `Deref`/`DerefMut` for transparent field-level change tracking across all entity types
- Introduce `TrackedMap<K,V>` (IndexMap-backed) for collection-level change tracking (inserted, modified, removed keys) — used in EntityStore and for workflow steps internally
- Add a `#[derive(Tracked)]` proc macro that generates tracked struct variants and `From<Plain>` conversion impls for each entity
- EntityStore internals change from `HashMap<String, Entity>` to `TrackedMap<String, TrackedEntity>`, with a public API that accepts and returns plain types
- Add `EntityStore::drain_changes()` producing a substrate-agnostic `ChangeSet` of flat `EntityChange` entries (path, kind, id, op with dirty field names)
- **BREAKING**: `Substrate::persist()` signature changes from `persist(&self, store: &EntityStore)` to `persist(&self, changeset: &ChangeSet)` — trait implementors receive a pre-built changeset instead of the full store
- RepoSubstrate `persist()` uses LCA-based atomic directory swap scoped to the smallest subtree containing all changes

## Capabilities

### New Capabilities
- `change-tracking`: Deep pervasive field-level change tracking via `Tracked<T>` and `TrackedMap<K,V>` primitives, with a derive macro for generating tracked entity variants
- `incremental-persistence`: Substrate-agnostic `ChangeSet` production from tracked entities, and LCA-based atomic persistence in RepoSubstrate

### Modified Capabilities
- `entity-store`: EntityStore internals change to use tracked types; public API accepts plain entities at insertion boundary and converts internally; new `drain_changes()` method produces `ChangeSet`
- `storage-layer`: `Substrate::persist()` signature changes to accept `&ChangeSet` instead of `&EntityStore`

## Impact

- **New crate**: `pari-macros` (proc-macro crate) for `#[derive(Tracked)]`
- **New dependency**: `indexmap` for ordered key-value storage in `TrackedMap`
- **Modified modules**: `src/schema/store.rs` (EntityStore internals), `src/substrate/mod.rs` (Substrate trait signature), `src/substrate/repo/storage.rs` (RepoSubstrate incremental persist implementation)
- **New modules**: `src/tracked.rs` (Tracked<T>, TrackedMap<K,V>), `src/substrate/changeset.rs` (ChangeSet, EntityChange types)
- **Test impact**: Existing tests continue to work — plain struct construction is unchanged. EntityStore tests need updates for new insertion API. Substrate tests need updates for new `persist()` signature.
- **Schema impact**: None — JSON schemas are generated from plain structs which remain unchanged.
