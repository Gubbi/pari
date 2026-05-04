# src/store — Store Layer

Formal `store` layer: in-memory tracked entity state, `StoreServer`/`Store` orchestration flow, the two dispatch boundaries the layer exposes, the JSON ↔ tracked pipeline, checkout and persist lifecycles, and load orchestration.

Authoritative design doc: [docs/design/layers/store.md](/Users/vinuth/code/pari/docs/design/layers/store.md). When this file and the design doc disagree, the design doc wins.

## Local Orientation

- Stateless orchestrator — substrate calls, JSON ↔ tracked pipeline, validation invocation through per-request workspace, load/persist sequencing; the workspace-facing `Dispatcher` impl: [store_server.rs](/Users/vinuth/code/pari/src/store/store_server.rs).
- State-custodian actor — sole owner of `entities`, `added`, `modified`, `removed`, `checked_out`; emits `PrimitiveError` only: [store.rs](/Users/vinuth/code/pari/src/store/store.rs).
- Dispatch trait boundaries: workspace-facing `Dispatcher` (carries `WorkspaceRequest` / `WorkspaceResponse`) and server-facing `StoreDispatcher` (carries `StoreRequest` / `StoreResponse`).
- Workspace-facing wire types: [lib/workspace_request.rs](/Users/vinuth/code/pari/src/store/lib/workspace_request.rs).
- Store-side wire types and message envelope: [lib/store_request.rs](/Users/vinuth/code/pari/src/store/lib/store_request.rs).
- Store → substrate persist handoff enum: [lib/change.rs](/Users/vinuth/code/pari/src/store/lib/change.rs).

## What Does Not Live Here

- Caller-facing async API, viewer/editor handles, validation rules → `workspace`
- Asset layout, codecs, resolvers, load strategies, JSON encoding → `substrate`
- Cross-layer error classification and aggregation → `error`

If an edit starts to describe file layout, resolver logic, or validation rule authoring, it belongs in another layer.

## Conventions Worth Repeating Locally

- Type-erased throughout: every public method on `StoreServer`, every variant of `WorkspaceRequest`/`WorkspaceResponse`, and every variant of `StoreRequest`/`StoreResponse` speaks `AnyEntityRef` and `TrackedEntity` only. No method is generic over a typed `T: Entity` — typed↔erased conversion is a workspace concern.
- `StoreServer` is stateless orchestration (`ActivityError`); `Store` is the layer's only async actor and the state-custodian pure tier (`PrimitiveError` only).
- All caller operations flow workspace → `Dispatcher` → `StoreServer` → (`StoreDispatcher` ∨ `substrate` ∨ workspace-back-edge) → reply. The workspace back-edge runs validation through a per-request `Workspace::new(self_dispatcher.upgrade()...)`; `StoreServer` holds a `Weak<dyn Dispatcher>` to its own handle to break the cycle.
- Composition: `Store::start(spawn_fn)` returns `Arc<dyn StoreDispatcher>`; `StoreServer::start(substrate, store_dispatcher)` returns `Arc<dyn Dispatcher>`. Both live in [src/lib.rs](/Users/vinuth/code/pari/src/lib.rs)' top-level wiring path. The same composition is used in production and in tests.
- Insert and substrate-load share one JSON ↔ tracked pipeline: `json_to_tracked_state` (pure conversion) and `json_to_verified_tracked` (full pipeline including per-request workspace import + validate). Substrate returns `serde_json::Value`; the server wraps and validates before handing the result to `Store`.
- The store, not the caller, picks which `ValidationKind`s run at each operation — see the design doc's validation-decision table. Commit is unified: CrossEntity only (whole entity if newly added; dirty-fields-scoped otherwise).
- `EntityChange` is the only type substrates see from the store's change-tracking; they must not depend on `Store` internals.
- Dirty state resets only after `substrate.persist` succeeds — a substrate error leaves change lists intact for retry.
- Lifecycle preconditions are enforced inside the actor — duplicate insert returns `EntityAlreadyExists`; commit on a ref that was never checked out returns `EntityNotCheckedOut`. The "checkout before mutate, commit only after checkout" contract is not a permissive convention.
- Vocabulary: `revert` rolls an entity back to its last persisted state; `forget` drops a clean entity's loaded fields, leaving a stub. Both are gated on the entity not being checked out.
