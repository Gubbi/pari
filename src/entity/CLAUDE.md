# src/entities — Entity Layer Plain Definitions

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

- Plain Rust structs and enums for domain entities.
- `#[derive(pari_macros::Entity)]` on those plain structs.
- Entity-local field declarations and entity-local default derives.
- Embedded workflow step entity definitions in [src/entities/workflow.rs](/Users/vinuth/code/pari/src/entities/workflow.rs), [src/entities/task.rs](/Users/vinuth/code/pari/src/entities/task.rs), and [src/entities/relay.rs](/Users/vinuth/code/pari/src/entities/relay.rs).

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

Do not document or reintroduce older helper names such as `with_value`.
