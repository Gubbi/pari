# src/workspace — Workspace Layer

Formal `workspace` layer: caller-facing async API over the entity server.

Authoritative design doc: [docs/design/layers/workspace.md](/Users/vinuth/code/pari/docs/design/layers/workspace.md). When this file and the design doc disagree, the design doc wins.

## Local Orientation

- Typed operations keyed by `AnyEntityRef`, plus the generic `checkout<T>` that returns the entity's per-type `Delegate`: [client.rs](/Users/vinuth/code/pari/src/workspace/client.rs).
- Pure entity-server dispatch helper: [lib/request.rs](/Users/vinuth/code/pari/src/workspace/lib/request.rs).
- Generated accessors (on `TrackedX`), per-entity `XDelegate` struct + setters + `commit` / `undo_checkout` — `#[derive(Entity)]` output: `generate_workspace_parts` in [pari-macros/src/workspace_codegen.rs](/Users/vinuth/code/pari/pari-macros/src/workspace_codegen.rs).

## What Does Not Live Here

- Actor state, message protocol, load/persist orchestration → `store`
- Asset layout, file formats, backend implementations → `substrate`
- Rule definition and execution → `validation`
- Cross-layer error classification and aggregation → `error`

If an edit starts to describe store dispatch, asset layout, or rule authoring, it belongs in another layer.

## Conventions Worth Repeating Locally

- Every entry point is `async fn` returning `Result<_, ActivityError>`.
- `lib::request` is infallible — it looks up the active `EntityServer` and dispatches the `StoreRequest`. Channel failures between the `EntityServer` and the `StoreManager` are classified inside the store and arrive as `ActivityError::store_unavailable("entity_server", …)` carried by `StoreResponse::Err`; orchestration sites forward those (and any other application-level error) unchanged.
- Mutation is gated by checkout at the type level. `EntityClient::resolve` returns a `TrackedEntity` (read-only — accessors only, `Clone`). `EntityClient::checkout::<T>(EntityRef<T, …>)` returns the typed `T::Delegate` (`XDelegate`) — setters live there, not on `TrackedX`. Delegates are not `Clone` and consume themselves on `commit(self)` / `undo_checkout(self)`. The compile-time guarantee is the contract: the only handle that can mutate or commit is the one returned by `checkout`.
- Setters are synchronous validation sites: they run `ValidationKind::Structural` + `ValidationKind::Semantic` against a candidate before swapping the `Arc<TrackedField<T>>`. Cross-entity validation runs at store-managed boundaries (commit, persist), not in setters.
- Transparent load covers both user accessors and validator-driven ref existence checks (`resolve`, `has_ref`).
- Do not document removed concepts: `workspace/error.rs`, `workspace/tracked_entity.rs` (both removed; operation errors flow via `ActivityError`, lifecycle methods now live on `XDelegate`).
