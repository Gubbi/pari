# src/store — Store Layer Actor And State

## Ownership

This directory belongs to the formal `store` layer.

It owns:

- in-memory tracked entity state
- actor message flow
- checkout and undo lifecycle
- load orchestration inside the actor
- persist orchestration and the store-to-substrate handoff

The authoritative design docs for this area live under [docs/design/store_layer/](/Users/vinuth/code/pari/docs/design/store_layer/) plus the store-owned load docs in [docs/design/workspace_layer/load/](/Users/vinuth/code/pari/docs/design/workspace_layer/load/).

## Module Map

- [src/store/server.rs](/Users/vinuth/code/pari/src/store/server.rs): `EntityServer`, sender management, `init()`, and test-scoped `with()`
- [src/store/state.rs](/Users/vinuth/code/pari/src/store/state.rs): `Store<S>` state machine and orchestration
- [src/store/message.rs](/Users/vinuth/code/pari/src/store/message.rs): internal request/response message types
- [src/store/change.rs](/Users/vinuth/code/pari/src/store/change.rs): `EntityChange<'a>` persistence handoff enum

## Current Core Types

- Type-erased entity wrapper: `TrackedEntity`
- Persist handoff type: `EntityChange<'a>`
- Channel boundary failure type: `StoreError` in [src/store_error.rs](/Users/vinuth/code/pari/src/store_error.rs)

Do not reintroduce stale names such as `StoreEntity` or `StoreEntityChange`.

## Boundary Rules

- `workspace` owns the public async API and operation-level error types.
- `store` owns the internal `StoreRequest` / `StoreResponse` protocol and actor execution.
- `substrate` owns persistence contracts and storage details.
- `validation` owns rule execution logic, but the store decides when validations run.

That means:

- no caller-facing ergonomics should be added here if they belong in `workspace`
- no file layout, codec, or resolver logic should be added here
- no new validation rules should be authored here

## Test Helper

`EntityServer::with(substrate, || async { ... })` is the current isolated test entry point. Do not document `with_test()` or other removed helpers.

## Persist Path

The store exposes changes lazily via `Store::changes()` as `EntityChange<'_>` values and passes them to the substrate. The substrate may depend on that explicit handoff type, but should not depend on store actor internals.
