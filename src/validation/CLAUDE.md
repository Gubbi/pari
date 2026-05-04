# validation — Workspace's Validation Sub-Area

This directory hosts the validation sub-area of the `workspace` layer: per-entity rule schemas, the pure runner that dispatches them, and the structural / semantic / cross-entity rule primitives themselves.

Authoritative design doc: [docs/design/layers/validation.md](/Users/vinuth/code/pari/docs/design/layers/validation.md), with [docs/design/layers/workspace.md](/Users/vinuth/code/pari/docs/design/layers/workspace.md) covering how the runner is invoked through workspace-bound handles. When this file and the design docs disagree, the design docs win.

## Local Orientation

- Schema types (`ValidationSchema`, `ValidatableTracked`, rule type aliases, field-selection check): [lib/schema.rs](/Users/vinuth/code/pari/src/validation/lib/schema.rs).
- Pure runner — walks `(field, kind)` pairs, accumulates `PrimitiveError`s: [lib/runner.rs](/Users/vinuth/code/pari/src/validation/lib/runner.rs).
- Rule kind enum (`Structural` / `Semantic` / `CrossEntity`): [kind.rs](/Users/vinuth/code/pari/src/validation/kind.rs).
- Per-entity schemas and rule primitives: [lib/rules/](/Users/vinuth/code/pari/src/validation/lib/rules).
- The `Validator` orchestration type and the static `LazyLock<ValidationRuleSet>` registry it stamps: workspace's runner host. Reachable through `XViewer::validate` / `validate_with` (and `XEditor::*` via `Deref`).

## What Does Not Live Here

- Caller-facing async API, viewer/editor handles, dispatcher composition → workspace at the layer root.
- In-memory state, dispatch flow, load/persist orchestration → `store`.
- Persistence layout or asset codecs → `substrate`.

## Conventions Worth Repeating Locally

- All rule bodies emit `PrimitiveError`. Orchestration wrapping into `ActivityError` happens once, on the path through `Validator::run` (called via the viewer's `validate` / `validate_with`).
- `InvalidValidationFieldSelection` → `pari_invariant_violation` (programmer bug). Everything else → `validation_failed`.
- Structural rules are sync and value-only. Semantic rules are async and entity-local — they receive `&XViewer<'_, T>` and read sibling fields via the viewer's typed accessors. Cross-entity rules also receive `&XViewer<'_, T>` and resolve other entities via `viewer.workspace().resolve(other_ref).await` / `has_ref(other_ref).await`.
- Use the `ref_check_rule!` macro for plain ref-existence checks on a field; reserve hand-written cross-entity rules for more elaborate checks (hook input binding, cycle detection, embedded-tree shape).
- Embedded entities (`Task`, `Relay`, `EmbeddedWorkflow`) cross-entity-validate their `entity_ref.parent` via the `parent_exists` helper in [lib/rules/cross_entity/common.rs](/Users/vinuth/code/pari/src/validation/lib/rules/cross_entity/common.rs). An embedded entity's parent must exist in the store at insert time.
- Per-entity schema builders live next to the entity's rule primitives in `lib/rules/<entity>.rs` and are dispatched by `#[derive(Entity)]`'s generated `validation_schema()`.
- Each entity's rules register into the process-wide `ValidationRuleSet` consumed by `Validator`. Adding a new entity's rules updates the registry once; every workspace picks them up.
