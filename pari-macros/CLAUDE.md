# pari-macros — Proc-Macro Support For Formal Layers

## Ownership

`pari-macros` is support code, not a formal architecture layer.

Every macro here should be explained in terms of the formal layer that owns the generated behavior:

- `entity` for tracked entity identity and serde support
- `workspace` for generated async accessors/setters and operation wiring
- `store` for tracked-wrapper helper methods used by the actor
- `validation` for validation dispatch glue
- `error` for error classification and telemetry derives

Do not describe this crate as a separate architectural home for runtime behavior.

## Macro Map

- `#[derive(Entity)]`: multi-layer generation across `entity`, `workspace`, and `validation`
- `entity_registry!`: generated aggregate types and dispatch used across `entity`, `store`, and `substrate`
- `#[derive(ErrorCompose)]`: `error`-layer classification derive
- `#[derive(OTelEmit)]`: `error`-layer telemetry derive

## Current Naming

- Generated type-erased tracked wrapper: `TrackedEntity`
- Store persist handoff referenced by generated code: `EntityChange`
- Setter-side field mutation helper: `TrackedField::mutated`
- Load/deserializer helper: `TrackedField::initialize`

Avoid documenting removed names such as `StoreEntity` or `TrackedField::with_value`.

## Editing Guidance

- Keep macro output aligned with the owning layer's design docs.
- If a generation concern spans multiple formal layers, document that split explicitly instead of inventing a generic "macro layer."
- Before changing generated API shape, check the relevant design docs under [docs/design/](/Users/vinuth/code/pari/docs/design/README.md).
