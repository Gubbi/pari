# src/entity — Entity Layer

Formal `entity` layer: identity, plain entity definitions, tracked-field primitive.

Authoritative design doc: [docs/design/layers/entities.md](/Users/vinuth/code/pari/docs/design/layers/entities.md). When this file and the design doc disagree, the design doc wins.

## Local Orientation

- Identity: [entity_ref.rs](/Users/vinuth/code/pari/src/entity/entity_ref.rs), [parent_kind.rs](/Users/vinuth/code/pari/src/entity/parent_kind.rs), [entity_trait.rs](/Users/vinuth/code/pari/src/entity/entity_trait.rs).
- Registry — the one `entity_registry!` invocation that wires every entity into cross-layer dispatch: [entity_kind.rs](/Users/vinuth/code/pari/src/entity/entity_kind.rs).
- Plain entities: [entities/](/Users/vinuth/code/pari/src/entity/entities).
- Shared value types embedded in entities: [types.rs](/Users/vinuth/code/pari/src/entity/types.rs).
- Tracked-field primitive: [tracked/tracked_field.rs](/Users/vinuth/code/pari/src/entity/tracked/tracked_field.rs).
- Uniform ref extraction: [collect_refs.rs](/Users/vinuth/code/pari/src/entity/collect_refs.rs).

## What Does Not Live Here

- Caller-facing API shaping → `workspace`
- Actor / message flow / checkout lifecycle → `store`
- Persistence layout, asset schemas, backends → `substrate`
- Rule definition and runner flow → `validation`
- Cross-cutting error classification → `error`

If an edit starts to discuss store dispatch, asset writes, or rule execution order, it probably belongs in another layer.

## Conventions Worth Repeating Locally

- Top-level ref: `EntityRef::new(id)`. Embedded ref: `EntityRef::with_parent(id, parent)`.
- Parent is part of semantic identity — two embedded entities with the same id under different parents are distinct. Do not reintroduce workflow-id-only helpers.
- Tracked field paths: `TrackedField::initialize(value)` for load/deserializer, `Arc::new(TrackedField::mutated(value))` for COW setter replacement.
- `Step` is not an entity — no `EntityRef`, no `#[derive(Entity)]`.
- `#[derive(Entity)]` generates behavior for multiple layers (`entity`, `workspace`, `validation`). Ownership of the generated items follows the consuming layer, not the macro crate.
- Per-entity `<Name>Delegate` (workspace-owned) is generated alongside `Tracked<Name>` (entity-owned). Setters and `commit` / `undo_checkout` live on the delegate; accessors live on the tracked companion. The `Entity::Delegate` associated type plus the `take` / `into_tracked_entity` methods on the trait round-trip the tracked form through delegate construction.
- New entity fields cannot use struct-keyed maps — the substrate intermediate is `serde_json::Value`, which only allows string keys. Use `Vec<(K, V)>` plus a uniqueness validation rule. See `docs/design/layers/entities.md` *Authoring Constraints*.
- Do not document removed concepts: `#[derive(Tracked)]`, `TrackedMap`, `TrackedField::with_value`, `StoreEntity`.
