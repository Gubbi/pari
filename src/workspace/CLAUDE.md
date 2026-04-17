# src/workspace — Workspace Layer Caller API

## Ownership

This directory belongs to the formal `workspace` layer.

It owns:

- caller-facing async operations via `EntityClient`
- operation error types returned to callers
- request helper glue over the store actor
- tracked-entity convenience methods such as `commit()` and `undo_checkout()`

The authoritative design docs for this area live under [docs/design/workspace_layer/](/Users/vinuth/code/pari/docs/design/workspace_layer/).

## Module Map

- [src/workspace/client.rs](/Users/vinuth/code/pari/src/workspace/client.rs): `EntityClient`
- [src/workspace/error.rs](/Users/vinuth/code/pari/src/workspace/error.rs): operation-level errors
- [src/workspace/protocol.rs](/Users/vinuth/code/pari/src/workspace/protocol.rs): request helper over `EntityServer`
- [src/workspace/tracked_entity.rs](/Users/vinuth/code/pari/src/workspace/tracked_entity.rs): `TrackedEntity::commit()` and `TrackedEntity::undo_checkout()`

## Boundary Rules

- `workspace` owns caller ergonomics, not actor internals.
- `store` owns `StoreRequest`, `StoreResponse`, actor state, and orchestration.
- `workspace` should not absorb persistence layout or substrate mechanics.
- `workspace` may trigger store-owned load orchestration, but does not own that algorithm.

## Current API Shape

- `EntityClient::{resolve, insert, remove, checkout, load, ensure_mutable, persist, undo_commit, unload}`
- operation errors live here: `CheckoutError`, `CommitError`, `LoadError`, `PersistError`, `ResolveError`, `UndoError`
- channel-level failure comes from `StoreError::Unavailable` and is wrapped into workspace operation errors as `StoreUnavailable(...)`

If documentation here starts describing actor state machines or persistence asset mapping, it has crossed out of the workspace layer.
