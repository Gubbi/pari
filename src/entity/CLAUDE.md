# src/entity — Entity Layer Identity And Plain Definitions

## Ownership

This directory belongs to the formal `entity` layer.

It owns plain domain entity shapes only:

- `Role`
- `Hook`
- `Team`
- `ArtifactKind`
- `Workflow`
- `ReusableWorkflow`
- `Task`
- `Relay`
- `EmbeddedWorkflow`
- supporting plain value enums/structs that live with those entities

The authoritative design docs for this area live under [docs/design/entity_layer/](/Users/vinuth/code/pari/docs/design/entity_layer/).

## What Lives Here

- Entity identity and typed refs in [src/entity/mod.rs](/Users/vinuth/code/pari/src/entity/mod.rs), [src/entity/entity_ref.rs](/Users/vinuth/code/pari/src/entity/entity_ref.rs), and [src/entity/parent_kind.rs](/Users/vinuth/code/pari/src/entity/parent_kind.rs).
- Plain Rust structs and enums for domain entities under [src/entity/entities/](/Users/vinuth/code/pari/src/entity/entities).
- `#[derive(pari_macros::Entity)]` on those plain structs.
- Entity-local field declarations and entity-local default derives.
- Shared entity-layer value types in [src/entity/types.rs](/Users/vinuth/code/pari/src/entity/types.rs).
- Tracked-field primitives in [src/entity/tracked/](/Users/vinuth/code/pari/src/entity/tracked).
- Embedded workflow entity definitions in [src/entity/entities/workflow.rs](/Users/vinuth/code/pari/src/entity/entities/workflow.rs), [src/entity/entities/task.rs](/Users/vinuth/code/pari/src/entity/entities/task.rs), and [src/entity/entities/relay.rs](/Users/vinuth/code/pari/src/entity/entities/relay.rs).

## What Does Not Live Here

- Caller-facing API shaping: `workspace`
- Actor/message flow or checkout lifecycle: `store`
- Persistence layout, asset schemas, codecs, resolvers, executors: `substrate`
- Validation orchestration or shared rule runners: `validation`
- Cross-cutting error classification: `error`

If a change starts to discuss actor requests, substrate assets, or validation execution order, it probably belongs outside this directory.

## Identity Rules

- Top-level entities use `EntityRef<T>`, which is `EntityRef<T, NoParent>`.
- Embedded entities use `EntityRef<T, WorkflowParent>`.
- Construct top-level refs with `EntityRef::new(id)`.
- Construct embedded refs with `EntityRef::with_parent(id, parent)`.
- `WorkflowParent` is a real parent hierarchy, not just a workflow-id string.

`Step` is not an entity. It references embedded entity refs but does not itself implement `Entity`.

## Generated Behavior From `#[derive(Entity)]`

The derive macro generates behavior used by multiple formal layers:

- `entity`: tracked companion struct and entity identity glue
- `workspace`: async field accessors and setters
- `validation`: tracked-entity validation dispatch

Keep that ownership split in mind when editing entity definitions. The macro mechanism lives in `pari-macros`, but the generated behavior still belongs to the owning formal layer.

## Change Tracking Expectations

Generated tracked companions wrap fields in `Arc<TrackedField<T>>`.

- load/deserializer path: `TrackedField::initialize(value)`
- mutation path: replace the field with `Arc::new(TrackedField::mutated(value))`

Do not document or reintroduce removed generic tracking concepts such as `#[derive(Tracked)]`, `TrackedMap`, or older helpers such as `with_value`.
