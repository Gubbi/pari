# src/workspace — Workspace Layer

Formal `workspace` layer: caller-facing async API over the entity server.

Authoritative design doc: [docs/design/layers/workspace.md](/Users/vinuth/code/pari/docs/design/layers/workspace.md). When this file and the design doc disagree, the design doc wins.

## Local Orientation

- Typed operations keyed by `AnyEntityRef`: [client.rs](/Users/vinuth/code/pari/src/workspace/client.rs).
- Checked-out entity methods (`commit`, `undo_checkout`): [tracked_entity.rs](/Users/vinuth/code/pari/src/workspace/tracked_entity.rs).
- Pure actor round-trip helper: [lib/request.rs](/Users/vinuth/code/pari/src/workspace/lib/request.rs).
- Generated accessors and setters — `#[derive(Entity)]` output: `generate_accessors_and_setters` in [pari-macros/src/workspace_codegen.rs](/Users/vinuth/code/pari/pari-macros/src/workspace_codegen.rs).

## What Does Not Live Here

- Actor state, message protocol, load/persist orchestration → `store`
- Asset layout, file formats, backend implementations → `substrate`
- Rule definition and execution → `validation`
- Cross-layer error classification and aggregation → `error`

If an edit starts to describe actor requests, asset layout, or rule authoring, it belongs in another layer.

## Conventions Worth Repeating Locally

- Every entry point is `async fn` returning `Result<_, ActivityError>`.
- Channel failures in `lib::request` emit `PrimitiveError` and are wrapped into `ActivityError::store_unavailable("entity_server", …)` at the orchestration sites. Application-level errors arrive via `StoreResponse::Err` and are forwarded unchanged.
- Setters are synchronous validation sites: they run `ValidationKind::Structural` + `ValidationKind::Semantic` against a candidate before swapping the `Arc<TrackedField<T>>`. Cross-entity validation runs at store-managed boundaries (commit, persist), not in setters.
- Transparent load covers both user accessors and validator-driven ref existence checks (`resolve`, `has_ref`).
- Do not document removed concepts: `workspace/error.rs` (file removed; operation errors are now aggregated via `ActivityError`).
