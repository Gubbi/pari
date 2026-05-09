# Pari — Codebase Guide

## What This Is

Rust library (`pari`) for workflow runtime behavior in hybrid human-agent teams.

The authoritative architecture reference is [docs/design/layers/layer-model.md](/Users/vinuth/code/pari/docs/design/layers/layer-model.md). Use the formal layer vocabulary from that doc when describing code ownership:

- `entity`
- `workspace`
- `store`
- `substrate`
- `error`

Validation lives inside `workspace` as a sub-area, not as a peer layer.

## Layer Map In Source

```text
src/
  entity/        entity-layer identity, plain entities, refs,
                 and tracked-field primitives                  -> see src/entity/CLAUDE.md
  workspace/     workspace-layer caller-facing async API,
                 viewer/editor handles, validation rules        -> see src/workspace/CLAUDE.md
  store/         store-layer server, state custodian,
                 dispatch boundaries                            -> see src/store/CLAUDE.md
  substrate/     substrate-layer persistence contracts/backends -> see src/substrate/CLAUDE.md
  error/         error-layer shared error infrastructure        -> see src/error/CLAUDE.md
  lib.rs         crate module wiring

pari-macros/
  proc-macro support for generated behavior across formal layers -> see pari-macros/CLAUDE.md

tests/
  integration coverage                                          -> see tests/CLAUDE.md
```

`schemas/` contains generated JSON Schema outputs for plain entity types. It is an output directory, not an architectural layer.

When working in a subtree, also look for a `CLAUDE.md` file in that directory or an ancestor within the repo. Treat nested guidance as additional local context.

## Current Naming And Ownership

- `TrackedEntity` is the type-erased tracked wrapper enum in `src/entity/mod.rs`. Construction is `pub(crate)` and reachable only through the store's JSON-to-tracked pipeline; substrate returns `serde_json::Value`, not `TrackedEntity`.
- `EntityChange` from `src/store/lib/change.rs` is the store-to-substrate persist handoff.
- Mutation is gated by typed workspace-bound handles. `Workspace::resolve(ref)` returns an `XViewer<'ws, T>` (read-only — typed async accessors, `validate` / `validate_with`). `Workspace::checkout::<T>(ref)` returns the typed `XEditor<'ws, T>` — setters and `commit(self)` / `undo_checkout(self)` live there. `XEditor` derefs to `XViewer` so reads work uniformly. Editors are not `Clone`. The contract is enforced at the type level.
- `EntityRef<T, P>::to_any_ref(&self)` is an instance method (no associated-fn form in current code).
- `workspace` owns caller-facing async operations, viewer/editor handles, validation rules and schemas, the runner, and the `Validator` type.
- `store` owns orchestration flow, in-memory state, checkout lifecycle, persist orchestration, the `Dispatcher` (workspace-facing) and `StoreDispatcher` (state-actor-facing) trait boundaries, and the JSON ↔ tracked conversion. `StoreServer` is the stateless workspace-facing dispatcher; `Store` is the layer's only async actor and sole state custodian.
- `substrate` owns the persistence trait, pipeline, schema-backed defaults, and concrete backends. The trait surface takes `&AnyEntityRef` (not `EntityKind`) and returns `serde_json::Value` for entity payloads.
- `error` owns cross-cutting classification and aggregation, including `PariError`.
- `pari-macros` is support code, not a separate architecture layer. Generated behavior belongs to the formal layer that owns that behavior.

## Composition

There are no globally-installed servers. Integrators wire components bottom-up:

- `Store::start(spawn_fn)` returns `Arc<dyn StoreDispatcher>`.
- `StoreServer::start(substrate, store_dispatcher)` returns `Arc<dyn Dispatcher>`.
- `Workspace::new(server_dispatcher)` returns a `Workspace`.

Multiple workspaces over the same server coexist; `StoreServer` itself constructs per-request workspaces internally for validation. The same composition is used in production and in tests.

## Entity Identity And Tracking

- `EntityRef<T, P>` uses `NoParent` for top-level entities and concrete parent kinds such as `WorkflowParent` for embedded workflow tree entities.
- Top-level refs use `EntityRef::new(id)`.
- Embedded refs use `EntityRef::with_parent(id, parent)`.
- Parent identity is part of semantic identity. Do not reintroduce workflow-id-only constructor helpers.
- `TrackedField<T>` paths: `loaded(value)` for the JSON-to-tracked pipeline (load and insert), `mutated(value)` for setter-side COW replacement.
- Author cross-referenced entity trees iteratively: insert parent shell with empty steps → insert each embedded child (its parent now exists) → modify parent's steps to point at the children. Recursive across embedded depth. See `docs/design/layers/entities.md` *Authoring Constraints*.

## Structural Conventions

The authoritative reference for these conventions is [docs/design/layers/layer-model.md — Within-Layer Structure](/Users/vinuth/code/pari/docs/design/layers/layer-model.md).

**Pure vs orchestration split**
Every layer has pure components in `lib/` (emit only `PrimitiveError`) and orchestration components at the layer root (wrap primitives into activity errors, or forward activity errors from deeper layers unchanged). `entity` is the sole exception — no orchestration layer, `PrimitiveError` at all boundaries.

**`mod.rs` files**
Contain only `mod` declarations and `pub use` re-exports — no logic, no `impl` blocks, no free functions.

**Runtime independence**
Production code (`src/`) must not depend on `tokio` or any other specific async runtime. Use `futures` channels, await futures, and route any spawning through a caller-provided `SpawnFn`. See [docs/design/framework.md](/Users/vinuth/code/pari/docs/design/framework.md) — *Runtime and Composition Integration*.

## Key Boundaries

- `entity` code should not absorb workspace, store, or substrate orchestration.
- `workspace` should stay focused on caller-facing APIs, viewer/editor handles, and validation rule authoring/execution.
- `store` may depend on `entity`, `substrate`, `workspace` (only via the validation back-edge through `Workspace::import_erased(...).validate_with(...)`), and `error`. It should not own persistence layout or caller ergonomics.
- `substrate` may depend on `entity`, `error`, and explicit store-owned handoff types such as `EntityChange`, but not on `StoreServer`/`Store` internals.
- Production layers must not depend on test code.

## Working Preferences

- Queue new topics and open questions in a repo-root `TODO.md` — create it when there are items to track, delete it when the queue empties.
- Work through queued items one at a time.
- Treat design docs as authoritative unless a real implementation constraint forces a design amendment.
- Keep concepts DRY across docs and local guidance. Link to the authoritative design doc instead of repeating long explanations.
- Commit at the end of each completed task so diffs stay easy to review task-by-task.
- Apply edits one file at a time. Do not preview diffs in chat before writing — the editor shows the diff natively. Pause after each file so the user can course-correct before the next one.

## Useful References

- Architecture: [docs/design/layers/layer-model.md](/Users/vinuth/code/pari/docs/design/layers/layer-model.md)
- Design index: [docs/design/README.md](/Users/vinuth/code/pari/docs/design/README.md)
- Root crate wiring: [src/lib.rs](/Users/vinuth/code/pari/src/lib.rs)
