# Pari — Codebase Guide

## What This Is

Rust library (`pari`) for workflow runtime behavior in hybrid human-agent teams.

The authoritative architecture reference is [docs/design/layers/layer-model.md](/Users/vinuth/code/pari/docs/design/layers/layer-model.md). Use the formal layer vocabulary from that doc when describing code ownership:

- `entity`
- `workspace`
- `store`
- `substrate`
- `validation`
- `error`
- `test`

## Layer Map In Source

```text
src/
  entity/        entity-layer identity, plain entities, refs,
                 and tracked-field primitives                  -> see src/entity/CLAUDE.md
  workspace/     workspace-layer caller-facing async API        -> see src/workspace/CLAUDE.md
  store/         store-layer server/state/orchestration          -> see src/store/CLAUDE.md
  substrate/     substrate-layer persistence contracts/backends -> see src/substrate/CLAUDE.md
  validation/    validation-layer rules and schemas             -> see src/validation/CLAUDE.md
  error/         error-layer shared error infrastructure        -> see src/error/CLAUDE.md
  lib.rs         crate module wiring

pari-macros/
  proc-macro support for generated behavior across formal layers -> see pari-macros/CLAUDE.md

tests/
  test-layer integration coverage                              -> see tests/CLAUDE.md
```

`schemas/` contains generated JSON Schema outputs for plain entity types. It is an output directory, not an architectural layer.

When working in a subtree, also look for a `CLAUDE.md` file in that directory or an ancestor within the repo. Treat nested guidance as additional local context.

## Current Naming And Ownership

- Use `TrackedEntity`, not `StoreEntity`, for the type-erased tracked wrapper enum in `src/entity/mod.rs`.
- Use `EntityChange` from `src/store/lib/change.rs` for the store-to-substrate persist handoff.
- Mutation is gated by typed `<Name>Delegate` handles. `EntityClient::resolve` returns a read-only `TrackedEntity`; `EntityClient::checkout::<T>(EntityRef<T, _>)` returns the typed `T::Delegate` (`RoleDelegate`, `WorkflowDelegate`, …) — setters and `commit(self)` / `undo_checkout(self)` live there. Delegates are not `Clone`. The contract is enforced at the type level.
- `workspace` owns caller-facing async operations and operation error types.
- `store` owns orchestration flow, in-memory state, checkout lifecycle, and persist orchestration. `EntityServer` is the stateless dispatcher workspace calls into; `StoreManager` is the singleton state-custodian actor and the store layer's only async actor.
- `substrate` owns the persistence trait, pipeline, schema-backed defaults, and concrete backends.
- `validation` owns rule definition and execution over tracked entities.
- `error` owns cross-cutting classification and aggregation, including `PariError`.
- `pari-macros` is support code, not a separate architecture layer. Generated behavior belongs to the formal layer that owns that behavior.

## Entity Identity And Tracking

- `EntityRef<T, P>` uses `NoParent` for top-level entities and concrete parent kinds such as `WorkflowParent` for embedded workflow tree entities.
- Top-level refs use `EntityRef::new(id)`.
- Embedded refs use `EntityRef::with_parent(id, parent)`.
- Parent identity is part of semantic identity. Do not reintroduce workflow-id-only constructor helpers.
- `TrackedField<T>` uses `initialize(value)` for write-once load/deserializer paths and `TrackedField::mutated(value)` for setter-side COW replacement.
- There is no separate `#[derive(Tracked)]`, `TrackedMap`, or generic tracked framework in the current design.

## Structural Conventions

The authoritative reference for these conventions is [docs/design/layers/layer-model.md — Within-Layer Structure](/Users/vinuth/code/pari/docs/design/layers/layer-model.md).

**Pure vs orchestration split**
Every layer has pure components in `lib/` (emit only `PrimitiveError`) and orchestration components at the layer root (wrap primitives into activity errors, or forward activity errors from deeper layers unchanged). `entity` is the sole exception — no orchestration layer, `PrimitiveError` at all boundaries.

**`mod.rs` files**
Contain only `mod` declarations and `pub use` re-exports — no logic, no `impl` blocks, no free functions.

**Runtime independence**
Production code (`src/`) must not depend on `tokio` or any other specific async runtime. Use `futures` channels, await futures, and route any spawning through a caller-provided `SpawnFn`. See [docs/design/framework.md](/Users/vinuth/code/pari/docs/design/framework.md) — *Runtime Independence*.

## Key Boundaries

- `entity` code should not absorb store, substrate, or validation orchestration.
- `workspace` should stay focused on caller-facing APIs over `EntityServer`.
- `store` may depend on `entity`, `substrate`, `validation`, and `error`, but should not own persistence layout or caller ergonomics.
- `substrate` may depend on `entity`, `error`, and explicit store-owned handoff types such as `EntityChange`, but not on Entity Server internals.
- `validation` defines rules; it should not own persistence or store orchestration flow.
- `test` may reach across production layers, but production layers must not depend on test code.

## Working Preferences

- Queue new topics and open questions in the repo-root [TODO.md](/Users/vinuth/code/pari/TODO.md).
- Work through queued items one at a time.
- Treat design docs as authoritative unless a real implementation constraint forces a design amendment.
- Keep concepts DRY across docs and local guidance. Link to the authoritative design doc instead of repeating long explanations.
- Commit at the end of each completed task so diffs stay easy to review task-by-task.

## Useful References

- Architecture: [docs/design/layers/layer-model.md](/Users/vinuth/code/pari/docs/design/layers/layer-model.md)
- Design index: [docs/design/README.md](/Users/vinuth/code/pari/docs/design/README.md)
- Root crate wiring: [src/lib.rs](/Users/vinuth/code/pari/src/lib.rs)
