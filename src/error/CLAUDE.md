# src/error — Error Layer Infrastructure

## Ownership

This directory belongs to the formal `error` layer.

It owns:

- classification enums
- `ErrorCompose`
- `OTelEmit`
- `BatchError<E>`
- top-level `PariError`

The authoritative design docs for this area live under [docs/design/error_layer/](/Users/vinuth/code/pari/docs/design/error_layer/).

## Current Core Types

- `FixDomain`
- `Recoverability`
- `Severity`
- `ErrorCompose`
- `OTelEmit`
- `BatchError<E>`
- `PariError`

`PariError` currently wraps workspace and validation failures using variants such as:

- `DefinitionRejected`
- `MutationFailed`
- `CheckoutFailed`
- `LoadFailed`
- `ResolveFailed`
- `SaveFailed`
- `SetterRejected`

Do not document the older umbrella variants that no longer exist.

## Boundary Rules

- This layer is cross-cutting infrastructure, not a home for store/workspace/substrate logic.
- Domain operation errors live with their owning layer, then compose into `PariError`.
- Error docs here should explain classification and aggregation behavior, not actor flow or persistence layout.

## Derive Guidance

`#[derive(ErrorCompose)]` and `#[derive(OTelEmit)]` live in `pari-macros`.

For delegating variants, use `#[compose(delegate)]`. Do not duplicate delegation metadata under `otel`.
