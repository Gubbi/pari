# src/store — Store Layer

Formal `store` layer: in-memory tracked entity state, actor flow, checkout and persist lifecycles, and load orchestration.

Authoritative design doc: [docs/design/layers/store.md](/Users/vinuth/code/pari/docs/design/layers/store.md). When this file and the design doc disagree, the design doc wins.

## Local Orientation

- Orchestration actor — substrate + validation sequencing, load/persist flow: [entity_server.rs](/Users/vinuth/code/pari/src/store/entity_server.rs).
- State-custodian actor — sole owner of `entities`, `added`, `modified`, `removed`, `checked_out`: [manager.rs](/Users/vinuth/code/pari/src/store/manager.rs).
- Workspace-facing message types: [lib/message.rs](/Users/vinuth/code/pari/src/store/lib/message.rs).
- Store → substrate persist handoff enum: [lib/change.rs](/Users/vinuth/code/pari/src/store/lib/change.rs).

## What Does Not Live Here

- Caller-facing async API and setter ergonomics → `workspace`
- Asset layout, codecs, resolvers, load strategies → `substrate`
- Rule definition and execution logic → `validation`
- Cross-layer error classification and aggregation → `error`

If an edit starts to describe file layout, resolver logic, or rule authoring, it belongs in another layer.

## Conventions Worth Repeating Locally

- `EntityServer` is orchestration (`ActivityError`); `StoreManager` is state-custodian pure tier (`PrimitiveError` only).
- All caller operations flow workspace → `EntityServer` → (`StoreManager` ∨ `substrate` ∨ `validation`) → reply.
- `EntityServer::with(substrate, || async { ... })` is the isolated test entry point. Do not document removed helpers.
- The store, not the caller, picks which `ValidationKind`s run at each operation — see the design doc's validation-decision table.
- `EntityChange` is the only type substrates see from the store's change-tracking; they must not depend on `StoreManager` internals.
- Dirty state resets only after `substrate.persist` succeeds — a substrate error leaves change lists intact for retry.
