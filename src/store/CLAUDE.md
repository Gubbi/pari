# src/store — Store Layer

Formal `store` layer: in-memory tracked entity state, `EntityServer`/`StoreManager` orchestration flow, checkout and persist lifecycles, and load orchestration.

Authoritative design doc: [docs/design/layers/store.md](/Users/vinuth/code/pari/docs/design/layers/store.md). When this file and the design doc disagree, the design doc wins.

## Local Orientation

- Stateless orchestrator — substrate + validation sequencing, load/persist flow; workspace's dispatch entry point: [entity_server.rs](/Users/vinuth/code/pari/src/store/entity_server.rs).
- State-custodian actor — sole owner of `entities`, `added`, `modified`, `removed`, `checked_out`: [manager.rs](/Users/vinuth/code/pari/src/store/manager.rs).
- Workspace-facing request/response types: [lib/message.rs](/Users/vinuth/code/pari/src/store/lib/message.rs).
- Store → substrate persist handoff enum: [lib/change.rs](/Users/vinuth/code/pari/src/store/lib/change.rs).

## What Does Not Live Here

- Caller-facing async API and setter ergonomics → `workspace`
- Asset layout, codecs, resolvers, load strategies → `substrate`
- Rule definition and execution logic → `validation`
- Cross-layer error classification and aggregation → `error`

If an edit starts to describe file layout, resolver logic, or rule authoring, it belongs in another layer.

## Conventions Worth Repeating Locally

- `EntityServer` is stateless orchestration (`ActivityError`); `StoreManager` is the layer's only async actor and the state-custodian pure tier (`PrimitiveError` only).
- All caller operations flow workspace → `EntityServer` → (`StoreManager` ∨ `substrate` ∨ `validation`) → reply.
- `pari::with(substrate, || async { ... })` is the isolated test entry point; `pari::init(substrate, spawn_fn)` wires the production `EntityServer`. Both live in [src/lib.rs](/Users/vinuth/code/pari/src/lib.rs).
- The store, not the caller, picks which `ValidationKind`s run at each operation — see the design doc's validation-decision table.
- `EntityChange` is the only type substrates see from the store's change-tracking; they must not depend on `StoreManager` internals.
- Dirty state resets only after `substrate.persist` succeeds — a substrate error leaves change lists intact for retry.
