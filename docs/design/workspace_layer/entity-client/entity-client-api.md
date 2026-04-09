# entity-client-api

**Workspace Layer → `workspace_layer/entity-client/`**

---

## Purpose

`EntityClient` is the Workspace Layer's stateless convenience wrapper over the Store Layer's message protocol. It provides one typed method per operation — all message construction is internal. Workspace Layer callers never construct messages or touch channels directly.

`EntityServer` is a singleton that exclusively owns all entity state. The `Store<S>` it owns is never exposed directly — all access goes through `EntityClient`.

---

## Internal Helpers

Not exposed outside this module. Used by `EntityClient` methods and by checked-out entity methods (`commit`, `undo_checkout`).

```rust
async fn request(req: StoreRequest) -> Result<StoreResponse, StoreError> {
    let (tx, rx) = oneshot::channel();
    EntityServer::sender()
        .send(StoreMessage::Request { request: req, reply: tx })
        .await
        .map_err(|_| StoreError::Unavailable)?;
    rx.await.map_err(|_| StoreError::Unavailable)?
}

async fn send(cmd: StoreCommand) -> Result<(), StoreError> {
    EntityServer::sender()
        .send(StoreMessage::Command(cmd))
        .await
        .map_err(|_| StoreError::Unavailable)
}
```

---

## EntityClient

Stateless struct. All methods are async.

```rust
pub struct EntityClient;

impl EntityClient {
    pub async fn resolve(any_ref: AnyEntityRef) -> Result<TrackedEntity, ResolveError> {
        match request(StoreRequest::Resolve { any_ref }).await
            .map_err(ResolveError::StoreUnavailable)?
        {
            StoreResponse::Entity(e) => Ok(e),
            StoreResponse::ResolveErr(e) => Err(e),
            _ => unreachable!(),
        }
    }

    pub async fn insert(entity: TrackedEntity) -> Result<(), StoreError> {
        send(StoreCommand::Insert(entity)).await
    }

    pub async fn remove(any_ref: AnyEntityRef) -> Result<TrackedEntity, StoreError> {
        match request(StoreRequest::Remove { any_ref }).await? {
            StoreResponse::Entity(e) => Ok(e),
            _ => unreachable!(),
        }
    }

    pub async fn checkout(any_ref: AnyEntityRef) -> Result<TrackedEntity, CheckoutError> {
        match request(StoreRequest::Checkout { any_ref }).await
            .map_err(CheckoutError::StoreUnavailable)?
        {
            StoreResponse::Entity(e) => Ok(e),
            StoreResponse::CheckoutError(e) => Err(e),
            _ => unreachable!(),
        }
    }

    pub async fn load(any_ref: AnyEntityRef, field: &str) -> Result<(), LoadError> {
        match request(StoreRequest::Load { any_ref, field: field.to_owned() }).await
            .map_err(LoadError::StoreUnavailable)?
        {
            StoreResponse::Unit => Ok(()),
            StoreResponse::LoadError(e) => Err(e),
            _ => unreachable!(),
        }
    }

    pub async fn ensure_mutable(any_ref: AnyEntityRef, field: &str) -> Result<(), LoadError> {
        match request(StoreRequest::EnsureMutable { any_ref, field: field.to_owned() }).await
            .map_err(LoadError::StoreUnavailable)?
        {
            StoreResponse::Unit => Ok(()),
            StoreResponse::LoadError(e) => Err(e),
            _ => unreachable!(),
        }
    }

    pub async fn persist() -> Result<(), PersistError> {
        match request(StoreRequest::Persist).await
            .map_err(PersistError::StoreUnavailable)?
        {
            StoreResponse::Unit => Ok(()),
            StoreResponse::PersistError(e) => Err(e),
            _ => unreachable!(),
        }
    }

    pub async fn undo_commit(any_ref: AnyEntityRef) -> Result<(), UndoError> {
        match request(StoreRequest::UndoCommit { any_ref }).await
            .map_err(UndoError::StoreUnavailable)?
        {
            StoreResponse::Unit => Ok(()),
            _ => unreachable!(),
        }
    }

    pub async fn unload(any_ref: AnyEntityRef) -> Result<(), UndoError> {
        match request(StoreRequest::Unload { any_ref }).await
            .map_err(UndoError::StoreUnavailable)?
        {
            StoreResponse::Unit => Ok(()),
            _ => unreachable!(),
        }
    }
}
```

Checked-out entity methods (`commit`, `undo_checkout`) use `request`/`send` directly via `EntityServer::sender()` — no `EntityClient` reference needed.

---

## API Summary

### Read Path

```rust
EntityClient::resolve(any_ref)              -> Result<TrackedEntity, ResolveError>
EntityClient::load(any_ref, field)          -> Result<(), LoadError>        // internal; called by accessors
task.name()                                 -> Result<&str, LoadError>      // transparent field load
```

`resolve` returns an owned clone — may be a stub if the entity has not been loaded. Field accessors trigger `EntityClient::load` transparently on first access; no explicit load call is needed by callers.

### Mutation Path

```rust
EntityClient::checkout(any_ref)             -> Result<TrackedEntity, CheckoutError>
EntityClient::insert(entity)                -> Result<(), StoreError>
EntityClient::remove(any_ref)               -> Result<TrackedEntity, StoreError>
EntityClient::ensure_mutable(any_ref, field) -> Result<(), LoadError>      // internal; called by setters

entity.commit()                             -> Result<(), CommitError>
entity.undo_checkout()                      -> Result<(), UndoError>
```

`remove` returns the evicted entity — call `insert` with it to undo. `checkout` is per-entity exclusive. `commit` validates, merges dirty fields, releases lock. `undo_checkout` releases lock and drops changes.

### Undo Path

```rust
EntityClient::undo_commit(any_ref) -> Result<(), UndoError>
EntityClient::unload(any_ref)      -> Result<(), UndoError>
```

`undo_commit` reverts to last persisted state (removes if `added`, stubs if `modified`). `unload` stubs a clean entity. Both require entity not checked out. See [store-entity-lifecycle](../../store_layer/entity-store/store-entity-lifecycle.md).

### Persist Path

```rust
EntityClient::persist()          -> Result<(), PersistError>
```

Fails if any checkouts are outstanding. Passes changes to substrate, resets dirty state on success.

---

## Async Convention

All public methods are async. No sync wrappers exist anywhere in this layer.

| API surface | Pattern |
|---|---|
| Field accessors (`task.name()`) | Async — transparent load on first access |
| Setters (`task.set_name()`) | Async — validates before writing |
| `EntityClient` methods | Async |
| `request`, `send` | Async — internal message helpers; not exposed to callers |

---

## Failure Contract

`EntityClient` never unwraps channel failures. If the actor channel is unavailable, `request()` / `send()` return `StoreError::Unavailable`, and the public API maps that into the operation's `StoreUnavailable(StoreError)` variant where one exists.

Application-level failures are carried inside `StoreResponse` so they survive the channel hop without being flattened into `StoreError`.

```
channel failure        => Err(<OperationError>::StoreUnavailable(StoreError::Unavailable))
application failure    => Ok(StoreResponse::<Operation>Err(e))   => Err(e)
successful operation   => Ok(StoreResponse::Entity/Unit)         => Ok(...)
```

## Thread Safety

`EntityServer` owns all entity state exclusively. No shared mutable references are issued. `Arc<TrackedField<T>>` snapshots are immutable after issue. Reads have eventual consistency — a resolved snapshot may lag behind the latest committed version.
