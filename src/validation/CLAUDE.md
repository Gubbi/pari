# src/validation — Validation Layer Rules And Schemas

## Ownership

This directory belongs to the formal `validation` layer.

It owns:

- per-entity `ValidationSchema<T>`
- structural, semantic, and cross-entity rules
- validation error data
- the shared validation runner over tracked entities

The authoritative design docs for this area live under [docs/design/validation_layer/](/Users/vinuth/code/pari/docs/design/validation_layer/).

## Current Model

- `Entity::validation_schema()` returns `&'static ValidationSchema<T>`
- rules operate on tracked entities, not plain entities
- `run_validations<T>(entity, fields, kinds)` accumulates `ValidationErrors`
- `run_validations_for_entity(&TrackedEntity, ...)` dispatches through the tracked wrapper

There is no `ValidationContext` type in the current source. Do not document one unless it is reintroduced in code.

## Rule Kinds

- `Structural`
- `Semantic`
- `CrossEntity`

Each kind is represented in [src/validation/error.rs](/Users/vinuth/code/pari/src/validation/error.rs) as `ValidationKind`.

## Boundary Rules

- Validation defines what is valid; it does not own when validations are triggered.
- The `store` layer decides when load-time, commit-time, and other validation runs happen.
- The `workspace` layer owns caller-facing error transport around those operations.
- The `substrate` layer must not absorb validation policy.

## Error Types

- `ValidationErrors`: aggregated plain data
- `FieldValidationError`
- `SetterError`: mutation-time failure returned by generated setters

`ValidationErrors` is plain data, not `ErrorCompose`.
