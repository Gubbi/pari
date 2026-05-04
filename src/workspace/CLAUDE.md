# src/workspace — Workspace Layer

Formal `workspace` layer: caller-facing async API, viewer/editor handles, and the validation sub-area (rules, schemas, runner, `Validator`).

Authoritative design doc: [docs/design/layers/workspace.md](/Users/vinuth/code/pari/docs/design/layers/workspace.md), with [docs/design/layers/validation.md](/Users/vinuth/code/pari/docs/design/layers/validation.md) covering the validation sub-area in detail. When this file and the design doc disagree, the design doc wins.

## Local Orientation

- `Workspace`, the bounded-session handle constructed over an `Arc<dyn Dispatcher>` to the store: [workspace.rs](/Users/vinuth/code/pari/src/workspace/workspace.rs).
- `XViewer<'ws, T>` and `XEditor<'ws, T>` — workspace-bound handles to typed tracked entities. The struct shells live in workspace; per-field accessors and setters are emitted by `#[derive(Entity)]`.
- `Validator` — workspace's runner host. Stamps the static rule registry; reachable through `XViewer::validate` / `validate_with`.
- Per-entity validation rules and schemas: `lib/rules/` (structural primitives, semantic rules, cross-entity rules, per-entity schema builders).
- Pure runner that walks `(field, kind)` pairs and accumulates `PrimitiveError`s: `lib/runner.rs`.
- Generated viewer/editor parts — accessor / setter / lifecycle generation for each entity: `generate_workspace_parts` in [pari-macros/src/workspace_codegen.rs](/Users/vinuth/code/pari/pari-macros/src/workspace_codegen.rs).

## What Does Not Live Here

- In-memory state, dispatch flow, load/persist orchestration → `store`
- Asset layout, file formats, backend implementations → `substrate`
- Cross-layer error classification and aggregation → `error`

If an edit starts to describe store dispatch, asset layout, or persistence layout, it belongs in another layer.

## Conventions Worth Repeating Locally

- Every public entry point on `Workspace`, `XViewer`, and `XEditor` is `async fn` returning `Result<_, ActivityError>` (except `Workspace::new`, `Workspace::import`, and the pass-through accessors on viewers).
- `Workspace::new(dispatcher)` is the only constructor. It is cheap — one `Arc` clone plus a static-reference stamp for the validator. Per-request construction inside server-side validation paths is fine.
- `Workspace`'s public methods take typed `EntityRef<T, T::Parent>` or plain `T` and return typed handles (`XViewer<'_, T>` / `XEditor<'_, T>` / `bool` / `()`). Workspace is the only site that converts between typed and type-erased forms; downstream layers see only `AnyEntityRef` and `TrackedEntity`.
- Mutation is gated by checkout at the type level. `Workspace::resolve` returns `XViewer` (read-only). `Workspace::checkout::<T>(EntityRef<T, _>)` returns `XEditor<'_, T>` — setters live there, not on `XViewer`. Editors are not `Clone` and consume themselves on `commit(self)` / `undo_checkout(self)`. The compile-time guarantee is the contract: the only handle that can mutate or commit is the one returned by checkout.
- `XEditor` derefs to `XViewer`, so all read accessors and `validate` / `validate_with` are reachable on an editor without separate generation.
- Setters are synchronous validation sites: they wrap a candidate `TrackedX<T>` in a transient `XViewer` over the same workspace and run `Structural` + `Semantic` rules through the validator before swapping the field's `Arc<TrackedField<T>>`. Cross-entity validation runs at server-driven gates (insert, commit, load), not in setters.
- Cross-entity rule bodies reach the store through `viewer.workspace().resolve(other_ref).await` and `viewer.workspace().has_ref(other_ref).await`. There is no separate `EntityClient` — every store hop flows through the same workspace handle the rule was invoked on.
- `Workspace::import(tracked)` wraps a transient `TrackedX<T>` as a viewer for validation outside the store; the type-erased counterpart `import_erased(tracked)` is `pub(crate)` and used by `StoreServer` for per-request validation workspaces.
- Channel failures between the workspace's dispatcher and the store layer arrive as `ActivityError::store_unavailable` carried by `WorkspaceResponse::Err`; orchestration sites forward those (and any other application-level error) unchanged.
- `Validator::new()` stamps a `&'static ValidationRuleSet` from a `LazyLock` registry built once per process. Per-workspace validators are effectively free.
