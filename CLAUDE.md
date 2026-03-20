# Pari — Codebase Guide

## What This Is

Rust library (`pari`) — a workflow runtime for hybrid human-agent teams. Two top-level modules:

- `src/schema/` — entity types, validation, entity store
- `src/substrate/` — persistence backend trait + repo (filesystem) implementation

---

## Module Map

```
src/
  schema/
    entities/    Role, Hook, Team, Workflow, SharedWorkflow
                 Task, Relay — embedded-only (no standalone entity, no top-level schema)
    store.rs     EntityStore — HashMap collections keyed by id; dual-purpose: validation context + persist input
    types.rs     Shared types (Raci, Artifact, HookInvocation, state types, Extensions, ...)
    validation.rs  validate() implementations per entity
  substrate/
    mod.rs       Substrate trait (persist only; load deferred to future proposal), SubstrateError
    repo/
      storage.rs   RepoSubstrate — atomic write via sibling .part/ dir, then fs::rename
      render.rs    Markdown+YAML-frontmatter renderers per entity type
```

---

## Key Decisions

**EntityStore invariant**: the entity being validated must NOT already be in the store. Callers enforce this.

**Task and Relay are embedded-only**: they live inside workflow steps, not as top-level entities. No standalone schema is generated for them.

**Atomic persistence**: RepoSubstrate writes to `<root>.part/`, then renames. On failure, `.part/` is cleaned up and errors are collected (not short-circuited).

**Extensions pattern**: every entity has an `extensions: Extensions` field (`HashMap<String, serde_json::Value>`) via `#[serde(flatten)]`. Only `x-` prefixed keys are allowed by schema.

**Schema generation**: `cargo xtask` drives `schemars` codegen into `schemas/`. Post-processing step adds `additionalProperties: false` to schemas with `patternProperties` (schemars 0.8 limitation with `#[serde(flatten)]`).

**Substrate::load is not yet defined**: the trait currently has `persist()` only. Loading from a substrate is a future proposal.

---

## Running Things

```sh
cargo test               # all tests (inline unit + schema coherence + storage integration)
cargo xtask              # regenerate schemas/ from Rust types
```

Tests: ~269 total across unit (inline), `tests/schema_coherence.rs`, `tests/storage_integration.rs`.

---

## Conventions

- IDs: kebab-case for Role/Team/Hook (e.g. `eng-lead`), CamelCase for Workflow/Task/Relay (e.g. `InitiativeWorkflow`)
- Inline unit tests in `#[cfg(test)]` blocks within each source file
- Integration tests in `tests/`
- No `pub use` re-exports at crate root — callers use full paths (`pari::schema::entities::role::Role`)
