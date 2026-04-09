# src/store — Actor-Based Entity Store

## Overview

`EntityServer` spawns a Tokio actor (`Store<S>`) that owns all in-memory entity state.
`EntityClient` provides a static async API that sends messages to the actor.
Isolation for tests is via `EntityServer::with_test()`, which installs a thread-local sender override.

---

## Store<S> Actor State

```rust
struct Store<S: Substrate> {
    entities:    HashMap<AnyEntityRef, StoreEntity>,  // all loaded entities (may be stubs)
    added:       HashSet<AnyEntityRef>,               // inserted this session, not yet persisted
    modified:    HashSet<AnyEntityRef>,               // committed with dirty fields, not yet persisted
    removed:     HashSet<AnyEntityRef>,               // removed this session, not yet persisted
    checked_out: HashSet<AnyEntityRef>,               // currently checked out (locked)
    substrate:   S,
}
```

**Stub**: a `StoreEntity` variant where all `TrackedField`s are uninitialized. Created by `StoreEntity::make_stub(&any_ref)`. Stubs are placeholders for referenced entities not yet loaded.

---

## store::Substrate trait

Separate from `substrate::Substrate`. Used by the actor.

```rust
pub trait Substrate: Send + Sync + 'static {
    fn exists(any_ref: AnyEntityRef) -> impl Future<Output = Result<bool, SubstrateError>> + Send;
    fn load(any_ref: AnyEntityRef, fields: Vec<String>) -> impl Future<Output = Result<StoreEntity, SubstrateError>> + Send;
    fn atomic_persist(changes: Vec<StoreEntityChange>) -> impl Future<Output = Result<(), Vec<SubstrateError>>> + Send;
}

pub enum StoreEntityChange {
    Added(StoreEntity),
    Modified(StoreEntity, Vec<&'static str>),  // dirty field names
    Removed(AnyEntityRef),
}
```

---

## EntityServer

```rust
EntityServer::init(substrate)                          // global init (production)
EntityServer::with_test(substrate, || async { ... })   // isolated test scope; installs thread-local override
```

---

## EntityClient — Public API

All methods are `async`. Error types are in `store/error.rs`.

| Method | Description | Returns |
|--------|-------------|---------|
| `insert(StoreEntity)` | Add new entity to store (fire-and-forget command) | `Result<(), StoreError>` |
| `resolve(AnyEntityRef)` | Get entity; loads from substrate if absent or stub | `Result<StoreEntity, StoreError>` |
| `checkout(AnyEntityRef)` | Get mutable copy + acquire lock | `Result<StoreEntity, CheckoutError>` |
| `remove(AnyEntityRef)` | Remove entity from store | `Result<StoreEntity, StoreError>` |
| `persist()` | Flush added/modified/removed to substrate | `Result<(), PersistError>` |
| `undo_commit(AnyEntityRef)` | Revert added entity (removes it) or modified entity (replaces with stub) | `Result<(), UndoError>` |
| `unload(AnyEntityRef)` | Replace clean entity with stub (enables re-load from substrate) | `Result<(), StoreError>` |

`StoreEntity` also has methods callable on the checked-out copy:
- `entity.commit().await` — merges dirty fields back into actor store, releases lock → `Result<(), CommitError>`
- `entity.undo_checkout().await` — discards changes, releases lock → `Result<(), UndoError>`

---

## Message Types (internal)

```rust
StoreRequest  // Resolve | Checkout | Commit | Remove | Persist | Load | UndoCommit | Unload
StoreCommand  // Insert(StoreEntity) | UndoCheckout { any_ref }
StoreResponse // Entity(StoreEntity) | Unit | CheckoutErr(CheckoutError) | PersistErr(PersistError)
StoreMessage  // Request { request, reply: oneshot::Sender<Result<StoreResponse, StoreError>> }
             //  | Command(StoreCommand)
```

Application-level errors (`CheckoutError`, `PersistError`) ride in `Ok(StoreResponse::XxxErr(e))`.
Channel-level failure is `Err(StoreError::Unavailable)`.

---

## Error Types (`store/error.rs`)

```
StoreError     — Unavailable (channel-level only)
CheckoutError  — AlreadyCheckedOut { entity_ref } | EntityNotFound { entity_ref } | Substrate(SubstrateError)
CommitError    — ValidationFailed { error_count, errors } | CrossReferenceCheckFailed(SubstrateError)
             |  StoreUnavailable(StoreError)
LoadError      — NotFound { entity_ref } | Substrate(SubstrateError) | ValidationFailed { error_count, errors }
UndoError      — WrongState | StoreUnavailable(StoreError)
PersistError   — PendingCheckouts { checked_out_count } | SubstrateErrors(BatchError<SubstrateError>)
ResolveError   — NotFound { entity_ref } | Substrate(SubstrateError)
```

---

## InMemorySubstrate (test helper)

```rust
let s = InMemorySubstrate::new();
s.seed(role_any_ref("pm"), StoreEntity::Role(tracked));   // pre-populate
EntityServer::with_test(s, || async { ... }).await;
```

---

## Core Jobs (see tests/core_jobs.rs)

1. **Read** — `resolve()` loads from substrate into memory
2. **Define** — `insert()` then `persist()` creates file on disk
3. **Update** — `resolve()` → `checkout()` → mutate → `commit()` → `persist()`
4. **Remove** — `resolve()` → `remove()` → `persist()` deletes file
5. **Save all pending** — single `persist()` flushes added + modified + removed
6. **Abandon in-progress** — `checkout()` → mutate → `undo_checkout()` (discards, releases lock)
7. **Rollback staged** — `checkout()` → `commit()` → `undo_commit()` (reverts to stub)
8. **Refresh from substrate** — `unload()` + `resolve()` pulls latest from disk
