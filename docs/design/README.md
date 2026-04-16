# Pari — Design Docs

The authoritative architecture reference for this tree is [architecture/layer-model](architecture/layer-model.md). The directory buckets below are the current document organization, while the formal layer vocabulary remains `entity`, `workspace`, `store`, `substrate`, `validation`, `error`, and `test`.

## Structure

| Current docs area | Formal layer focus | Contents |
|---|---|---|
| `architecture/` | architecture reference | Formal architectural model, layer ownership, dependency expectations |
| `entity_layer/` | `entity` | Field primitives, entity identity, value types, plain entities, tracked entities |
| `store_layer/` | `store` | Store structure, checkout/commit lifecycle, change tracking, persist phases |
| `workspace_layer/` | mostly `workspace` | Caller-facing API docs plus a historical `load/` bucket whose docs describe `store`-owned load orchestration triggered by workspace accessors |
| `substrate_layer/` | `substrate` | Substrate trait, asset pipeline, RepoSubstrate implementation |
| `validation_layer/` | `validation` | Validation API and per-entity implementations |
| `error_layer/` | `error` | Error taxonomy, composition, observability, batch errors, primitive-error design |
| `test_layer/` | `test` | Test-layer design docs and verification guidance |
| `codegen/` | not a formal layer | Implementation-support docs for generated behavior; each doc names its owning formal layer |

---

## architecture

- [layer-model](architecture/layer-model.md) — authoritative definition of the formal `entity`, `workspace`, `store`, `substrate`, `validation`, `error`, and `test` layers; also explains how the current design directories map onto that model

---

## entity

### field-primitives
- [tracked-field](entity_layer/field-primitives/tracked-field.md) — `TrackedField<T>`: `OnceLock<T>`, dirty flag, optional fields, Arc COW wrapping
- [cow-field-convention](entity_layer/field-primitives/cow-field-convention.md) — `Arc<TrackedField<T>>` as the COW wrapper: checkout cost, setter pattern, merge_dirty_into

### entity-identity
- [entity-trait](entity_layer/entity-identity/entity-trait.md) — `Entity` trait: KIND, VALIDATION_SCHEMA, Parent, Tracked; `TrackedEntity` companion trait
- [entity-kind-enum](entity_layer/entity-identity/entity-kind-enum.md) — `EntityKind` variants
- [parent-kind](entity_layer/entity-identity/parent-kind.md) — `ParentKind` trait, `NoParent`, workflow parents
- [entity-ref](entity_layer/entity-identity/entity-ref.md) — `EntityRef<T,P>`: structure, construction, id(), path()
- [entity-ref-hash-eq](entity_layer/entity-identity/entity-ref-hash-eq.md) — Hash+Eq via (KIND, id, parent chain)
- [entity-ref-serde](entity_layer/entity-identity/entity-ref-serde.md) — Wire format: `{id, type, parent?}`
- [any-entity-ref](entity_layer/entity-identity/any-entity-ref.md) — `AnyEntityRef` enum; one variant per entity kind

### value-types
- [extensions](entity_layer/value-types/extensions.md)
- [raci](entity_layer/value-types/raci.md)
- [hook-call](entity_layer/value-types/hook-call.md)
- [artifact](entity_layer/value-types/artifact.md)
- [state-entries](entity_layer/value-types/state-entries.md)
- [intercepts](entity_layer/value-types/intercepts.md)

### plain-entities
- [role](entity_layer/plain-entities/role.md)
- [hook](entity_layer/plain-entities/hook.md)
- [team](entity_layer/plain-entities/team.md)
- [workflow](entity_layer/plain-entities/workflow.md)
- [workflow-restructuring](entity_layer/plain-entities/workflow-restructuring.md)
- [workflow-variants](entity_layer/plain-entities/workflow-variants.md)
- [step-types](entity_layer/plain-entities/step-types.md)
- [task](entity_layer/plain-entities/task.md)
- [relay](entity_layer/plain-entities/relay.md)
- [artifact-kind](entity_layer/plain-entities/artifact-kind.md)

### tracked-entity
- [tracked-entity-pattern](entity_layer/tracked-entity/tracked-entity-pattern.md) — `TrackedField`, `Arc` COW, accessor generation
- [tracked-role](entity_layer/tracked-entity/tracked-role.md)
- [tracked-relay](entity_layer/tracked-entity/tracked-relay.md)
- [dirty-operations](entity_layer/tracked-entity/dirty-operations.md) — `has_dirty_fields()`, `reset_dirty()`, `dirty_fields()`
- [all-refs](entity_layer/tracked-entity/all-refs.md) — `all_refs()` for cross-entity ref collection
- [from-plain-entity](entity_layer/tracked-entity/from-plain-entity.md) — `From<PlainEntity>` conversion

---

## store

### checkout
- [store-checkout](store_layer/checkout/store-checkout.md) — `EntityClient::checkout()`: clone entity, mark checked_out
- [store-commit](store_layer/checkout/store-commit.md) — `entity.commit()` / `entity.undo_checkout()`: merge, validate, release
- [single-checkout-rule](store_layer/checkout/single-checkout-rule.md) — one checkout per entity; persist blocked if any outstanding

### entity-server
- [store-server](store_layer/entity-server/store-server.md) — `EntityServer` singleton, message protocol (StoreRequest/Command/Response), actor loop

### entity-store
- [store-structure](store_layer/entity-store/store-structure.md) — `Store<S>` struct, `TrackedEntity` enum, `Resolvable` trait, insert/remove/undo_commit/unload
- [store-resolve](store_layer/entity-store/store-resolve.md) — resolve: cache hit / stub creation / substrate existence check
- [store-has-ref](store_layer/entity-store/store-has-ref.md) — `has_ref()`: delegates to resolve, leaves stub in store
- [store-persist-phases](store_layer/entity-store/store-persist-phases.md) — pre-check, execute, reset phases
- [store-entity-lifecycle](store_layer/entity-store/store-entity-lifecycle.md) — full state machine: primary states, checkout overlay, operation preconditions, transitions

### change-tracking
- [entity-change-lists](store_layer/change-tracking/entity-change-lists.md) — `added`/`modified`/`removed` sets; new-then-remove and remove-then-new transition logic
- [entity-change-iterator](store_layer/change-tracking/entity-change-iterator.md) — `EntityChange` enum; `Store::changes()` lazy iterator
- [persist-dirty-reset](store_layer/change-tracking/persist-dirty-reset.md) — post-persist reset: dirty flags cleared, lists emptied

---

## workspace

`workspace_layer/` holds the caller-facing API docs. Its `load/` subdirectory remains a historical bucket, but those docs now describe `store`-owned load orchestration that workspace accessors and setters trigger.

### entity-client
- [entity-client-api](workspace_layer/entity-client/entity-client-api.md) — `EntityClient`: caller-facing async API over the store message boundary

### load
- [store-load-internal](workspace_layer/load/store-load-internal.md) — `store`-owned load handler inside `EntityServer`, invoked by workspace accessors
- [ensure-mutable](workspace_layer/load/ensure-mutable.md) — `store`-owned pre-mutation preparation flow invoked by workspace setters
- [progressive-loading-loop](workspace_layer/load/progressive-loading-loop.md) — `store`-owned multi-round load orchestration: prerequisites → fields → validate → merge

---

## substrate

### substrate-trait
- [load-strategy](substrate_layer/substrate-trait/load-strategy.md) — `LoadStrategy`: prerequisites, mutable_without_load; static per (EntityKind, field)
- [substrate-trait](substrate_layer/substrate-trait/substrate-trait.md) — `Substrate` trait: load, persist default impl, schema accessors
- [void-substrate](substrate_layer/substrate-trait/void-substrate.md) — no-op substrate for testing
- [error-types](substrate_layer/substrate-trait/error-types.md) — `SubstrateError`, `CheckoutError`, `CommitError`, `UndoError`, `LoadError`, `PersistError`

### pipeline
- [slot-and-asset-kind](substrate_layer/pipeline/slot-and-asset-kind.md)
- [field-mapping](substrate_layer/pipeline/field-mapping.md)
- [entity-schema](substrate_layer/pipeline/entity-schema.md)
- [substrate-schema-trait](substrate_layer/pipeline/substrate-schema-trait.md)
- [location-resolver](substrate_layer/pipeline/location-resolver.md)
- [codec](substrate_layer/pipeline/codec.md)
- [executor](substrate_layer/pipeline/executor.md)
- [asset-op-vocabulary](substrate_layer/pipeline/asset-op-vocabulary.md)
- [read-path](substrate_layer/pipeline/read-path.md)
- [write-path](substrate_layer/pipeline/write-path.md) — persist execution: EntityChange → AssetMapper → resolver → codec → executor

### repo-substrate
- [repo-slot](substrate_layer/repo-substrate/repo-slot.md)
- [repo-asset-kinds](substrate_layer/repo-substrate/repo-asset-kinds.md)
- [repo-location-resolver](substrate_layer/repo-substrate/repo-location-resolver.md)
- [repo-codec](substrate_layer/repo-substrate/repo-codec.md)
- [repo-executor](substrate_layer/repo-substrate/repo-executor.md) — LCA-based atomic swap via `.part/` staging
- [repo-entity-schemas](substrate_layer/repo-substrate/repo-entity-schemas.md)
- [repo-substrate-impl](substrate_layer/repo-substrate/repo-substrate-impl.md)
- [step-shorthand](substrate_layer/repo-substrate/step-shorthand.md)

---

## validation
- [validation-api](validation_layer/validation-api.md)
- [validate-shared](validation_layer/validate-shared.md)
- [validate-role](validation_layer/validate-role.md)
- [validate-hook](validation_layer/validate-hook.md)
- [validate-team](validation_layer/validate-team.md)
- [validate-task](validation_layer/validate-task.md)
- [validate-relay](validation_layer/validate-relay.md)
- [validate-workflow](validation_layer/validate-workflow.md)
- [validation-integration](validation_layer/validation-integration.md)
- [partial-validation](validation_layer/partial-validation.md)

---

## codegen support

`codegen/` is not a formal architecture layer. These docs explain how generation supports behavior owned by the formal layers, and each document names that ownership explicitly.
- [entity-kind-naming](codegen/entity-kind-naming.md)
- [entity-registry](codegen/entity-registry.md) — `entity_registry!` macro; generates `EntityKind`, `AnyEntityRef`, `TrackedEntity` enums, `load_strategy`

### async
- [async-accessor-variants](codegen/async/async-accessor-variants.md) — async accessor and setter generation; pure async, no sync counterparts; validators target `TrackedEntity` directly

### serde
- [tracked-entity-serde](codegen/serde/tracked-entity-serde.md) — OnceLock-aware Serialize/Deserialize; write-once merge; codec integration

---

## error
- [error-handling](error_layer/error-handling.md) — error taxonomy, `ErrorCompose`, `OTelEmit`, batch errors, client usage
- [primitive-errors](error_layer/primitive-errors.md) — Primitive Error design: derive-driven contract, auto-captured diagnostics, and standardized observability

---

## test

This layer currently has no dedicated design docs.

---

## cross-cutting
- [known-issues](known_issues.md) — explicitly deferred design gaps (KI-1 through KI-3)
