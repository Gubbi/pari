# src/validation — Validation Layer

Formal `validation` layer: per-entity rule schemas, the runner that
dispatches them, and the structural / semantic / cross-entity rule
primitives themselves.

Authoritative design doc: [docs/design/layers/validation.md](/Users/vinuth/code/pari/docs/design/layers/validation.md).
When this file and the design doc disagree, the design doc wins.

## Local Orientation

- Orchestration entry points (wrap pure runner into `ActivityError`):
  [runner.rs](/Users/vinuth/code/pari/src/validation/runner.rs).
- Pure runner (dispatches rules, accumulates `PrimitiveError`):
  [lib/runner.rs](/Users/vinuth/code/pari/src/validation/lib/runner.rs).
- Schema types (`ValidationSchema`, `ValidatableTracked`, rule type
  aliases, field-selection check):
  [lib/schema.rs](/Users/vinuth/code/pari/src/validation/lib/schema.rs).
- Rule kind enum (`Structural` / `Semantic` / `CrossEntity`):
  [kind.rs](/Users/vinuth/code/pari/src/validation/kind.rs).
- Per-entity schemas and rule primitives:
  [lib/rules/](/Users/vinuth/code/pari/src/validation/lib/rules).

## What Does Not Live Here

- When validations fire (setter / load / commit) — decided by
  `workspace` (generated setters) and `store` (`EntityServer`).
- Caller-facing error transport — `workspace` and `error`.
- Persistence layout or asset codecs — `substrate`.

## Conventions Worth Repeating Locally

- All rule bodies emit `PrimitiveError`. Orchestration wrapping into
  `ActivityError` happens once, at the top of `validation::runner`.
- `InvalidValidationFieldSelection` → `pari_invariant_violation`
  (programmer bug). Everything else → `validation_failed`.
- Structural rules are sync and value-only. Semantic rules are async
  and entity-local. Cross-entity rules are async and may call
  `EntityClient::has_ref`.
- Use the `ref_check_rule!` macro for plain ref-existence checks on a
  field; reserve hand-written cross-entity rules for more elaborate
  checks (hook input binding, cycle detection, embedded-tree shape).
- Embedded entities (`Task`, `Relay`, `EmbeddedWorkflow`) cross-entity-
  validate their `entity_ref.parent` via the `parent_exists` helper in
  [lib/rules/cross_entity/common.rs](/Users/vinuth/code/pari/src/validation/lib/rules/cross_entity/common.rs).
  An embedded entity's parent must exist in the store at insert time.
- Per-entity schema builders live next to the entity's rule primitives
  in `lib/rules/<entity>.rs` and are dispatched by
  `#[derive(Entity)]`'s generated `validation_schema()`.
