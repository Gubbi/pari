# src/error — Error Layer Infrastructure

## Ownership

This directory belongs to the formal `error` layer.

It owns:

- classification enums (`FixDomain`, `Recoverability`, `Severity`)
- the `ErrorCompose` trait and `as_error<E>` downcasting
- the `OTelEmit` trait
- the primitive diagnostics types (`ErrorLocation`, `PrimitiveDetail`, `ErrorLayer`)
- the centralized `PrimitiveError` enum
- the centralized `ActivityError` enum
- the top-level `PariError` umbrella

## Authoritative design

- L3 — [docs/design/layers/error-handling.md](/Users/vinuth/code/pari/docs/design/layers/error-handling.md)
- L4 — rustdoc co-located with the code in this directory and in
  [pari-macros/src/](/Users/vinuth/code/pari/pari-macros/src/)

This `CLAUDE.md` is an index for agents, not a source of truth. When it drifts
from the design doc or the code, the design doc + rustdoc win.

## Error Tiers

Pari's error chain is **Job → Activity → Primitive**. There is no Intermediary
Op tier — the hierarchy is framed in product / business language and is
deliberately independent of the code's component hierarchy.

## Key Types

- `PariError` — job-tier umbrella (`src/error/pari_error.rs`)
- `ActivityError` — centralized activity enum (`src/error/activity.rs`)
- `PrimitiveError` — centralized primitive enum (`src/error/primitive/primitive_errors.rs`)
- `ErrorCompose` / `OTelEmit` — the two derives every error participates in

## Derive Guidance

`#[derive(ErrorCompose)]` and `#[derive(OTelEmit)]` live in `pari-macros`.
`activity_errors! { ... }` and `primitive_errors! { ... }` are the declarative
entry points for authoring activity / primitive variants.

For delegating variants, use `#[compose(delegate)]`. Do not duplicate
delegation metadata under `otel`.

## Boundary Rules

- This layer is cross-cutting infrastructure, not a home for store / workspace
  / substrate logic.
- Pure `lib/` components in each layer emit `PrimitiveError` at point of
  failure; orchestration code wraps those into `ActivityError` variants.
- `PariError` is the only error type integrators should need to import.
