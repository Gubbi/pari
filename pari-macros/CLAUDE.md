# pari-macros — Proc-Macro Support For Formal Layers

## Ownership

`pari-macros` is support code, not a formal architecture layer.

Every macro here should be explained in terms of the formal layer that owns the generated behavior:

- `entity` for tracked entity state (`Tracked<Name>`), the `Entity` trait impl, and `EntityRef::to_any_ref` instance methods
- `workspace` for generated viewer / editor types and the per-field accessors / setters / lifecycle they expose, plus validation dispatch glue
- `store` for tracked-wrapper helper methods used by store internals
- `error` for error classification and telemetry derives

Do not describe this crate as a separate architectural home for runtime behavior.

## Macro Map

- `#[derive(Entity)]`: multi-layer generation across `entity` and `workspace`. For each entity it produces:
  - `Tracked<Name>` companion struct — state only, `pub(crate)` helpers, no async surface (entity).
  - `impl Entity for <Name>` with `KIND`, `Parent`, `Tracked`, `validation_schema()`, `extract`, `take`, `into_tracked_entity` (entity).
  - `impl TrackedFor for Tracked<Name>` (entity).
  - `EntityRef<Name, Parent>::to_any_ref(&self)` instance method (entity).
  - `XViewer<'ws, Name>` with per-field async accessors and `validate` / `validate_with` (workspace).
  - `XEditor<'ws, Name>` with `Deref<Target = XViewer<'ws, Name>>`, per-field setters, and `commit(self)` / `undo_checkout(self)` (workspace).
- `entity_registry!`: generated aggregate types and dispatch used across `entity`, `store`, `substrate`, and the validation sub-area inside workspace.
- `#[derive(ErrorCompose)]`: `error`-layer classification derive.
- `#[derive(OTelEmit)]`: `error`-layer telemetry derive.

## Current Naming

- Generated type-erased tracked wrapper: `TrackedEntity` (entity layer).
- Per-entity read handle (workspace-bound, lifetime-scoped): `XViewer<'ws, Name>`.
- Per-entity mutation handle (single-writer, not `Clone`, consumes self on lifecycle terminators): `XEditor<'ws, Name>`.
- Store persist handoff referenced by generated code: `EntityChange`.
- Setter-side field mutation helper: `TrackedField::mutated`.
- Field-level constructor used by the store's JSON-to-tracked pipeline: `TrackedField::loaded`.
- Workspace-facing dispatcher trait: `Dispatcher`. Server-facing dispatcher trait into the state custodian: `StoreDispatcher`.

## Editing Guidance

- Keep macro output aligned with the owning layer's design docs.
- If a generation concern spans multiple formal layers, document that split explicitly instead of inventing a generic "macro layer."
- Before changing generated API shape, check the relevant design docs under [docs/design/](/Users/vinuth/code/pari/docs/design/README.md).
