# Pari — Design Docs

## Structure

| Layer | Contents |
|---|---|
| `data_model/` | Field primitives, entity identity, value types, plain entities, tracked entities |
| `store_layer/` | Store structure, checkout/commit lifecycle, change tracking, persist phases |
| `workspace_layer/` | EntityClient API, field accessors/setters, loading |
| `substrate_layer/` | Substrate trait, asset pipeline, RepoSubstrate implementation |
| `validation/` | Validation API and per-entity implementations |
| `codegen/` | Derive macros, schema codegen |
| `error-handling/` | Error taxonomy, composition, observability, batch errors |

---

## data_model

### field-primitives
- [tracked-field](data_model/field-primitives/tracked-field.md) — `TrackedField<T>`: `OnceLock<T>`, dirty flag, optional fields, Arc COW wrapping
- [cow-field-convention](data_model/field-primitives/cow-field-convention.md) — `Arc<TrackedField<T>>` as the COW wrapper: checkout cost, setter pattern, merge_dirty_into

### entity-identity
- [entity-trait](data_model/entity-identity/entity-trait.md) — `Entity` trait: KIND, VALIDATION_SCHEMA, Parent, Tracked; `TrackedEntity` companion trait
- [entity-kind-enum](data_model/entity-identity/entity-kind-enum.md) — `EntityKind` variants
- [parent-kind](data_model/entity-identity/parent-kind.md) — `ParentKind` trait, `NoParent`, workflow parents
- [entity-ref](data_model/entity-identity/entity-ref.md) — `EntityRef<T,P>`: structure, construction, id(), path()
- [entity-ref-hash-eq](data_model/entity-identity/entity-ref-hash-eq.md) — Hash+Eq via (KIND, id, parent chain)
- [entity-ref-serde](data_model/entity-identity/entity-ref-serde.md) — Wire format: `{id, type, parent?}`
- [any-entity-ref](data_model/entity-identity/any-entity-ref.md) — `AnyEntityRef` enum; one variant per entity kind

### value-types
- [extensions](data_model/value-types/extensions.md)
- [raci](data_model/value-types/raci.md)
- [hook-call](data_model/value-types/hook-call.md)
- [artifact](data_model/value-types/artifact.md)
- [state-entries](data_model/value-types/state-entries.md)
- [intercepts](data_model/value-types/intercepts.md)

### plain-entities
- [role](data_model/plain-entities/role.md)
- [hook](data_model/plain-entities/hook.md)
- [team](data_model/plain-entities/team.md)
- [workflow](data_model/plain-entities/workflow.md)
- [workflow-restructuring](data_model/plain-entities/workflow-restructuring.md)
- [workflow-variants](data_model/plain-entities/workflow-variants.md)
- [step-types](data_model/plain-entities/step-types.md)
- [task](data_model/plain-entities/task.md)
- [relay](data_model/plain-entities/relay.md)
- [artifact-kind](data_model/plain-entities/artifact-kind.md)

### tracked-entity
- [tracked-entity-pattern](data_model/tracked-entity/tracked-entity-pattern.md) — `TrackedField`, `Arc` COW, accessor generation
- [tracked-role](data_model/tracked-entity/tracked-role.md)
- [tracked-relay](data_model/tracked-entity/tracked-relay.md)
- [dirty-operations](data_model/tracked-entity/dirty-operations.md) — `has_dirty_fields()`, `reset_dirty()`, `dirty_fields()`
- [all-refs](data_model/tracked-entity/all-refs.md) — `all_refs()` for cross-entity ref collection
- [from-plain-entity](data_model/tracked-entity/from-plain-entity.md) — `From<PlainEntity>` conversion

---

## store_layer

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

## workspace_layer

### entity-client
- [entity-client-api](workspace_layer/entity-client/entity-client-api.md) — `EntityClient`: `request`/`send` helpers; all typed async methods; thread safety

### load
- [store-load-internal](workspace_layer/load/store-load-internal.md) — internal load handler inside EntityServer
- [ensure-mutable](workspace_layer/load/ensure-mutable.md) — pre-mutation asset load; mutable_without_load; multi-asset entities
- [progressive-loading-loop](workspace_layer/load/progressive-loading-loop.md) — multi-round load: prerequisites → fields → validate → merge

---

## substrate_layer

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
- [validation-api](validation/validation-api.md)
- [validate-shared](validation/validate-shared.md)
- [validate-role](validation/validate-role.md)
- [validate-hook](validation/validate-hook.md)
- [validate-team](validation/validate-team.md)
- [validate-task](validation/validate-task.md)
- [validate-relay](validation/validate-relay.md)
- [validate-workflow](validation/validate-workflow.md)
- [validation-integration](validation/validation-integration.md)
- [partial-validation](validation/partial-validation.md)

---

## codegen
- [entity-kind-naming](codegen/entity-kind-naming.md)
- [entity-registry](codegen/entity-registry.md) — `entity_registry!` macro; generates `EntityKind`, `AnyEntityRef`, `TrackedEntity` enums, `load_strategy`

### async
- [async-accessor-variants](codegen/async/async-accessor-variants.md) — async accessor and setter generation; pure async, no sync counterparts; validators target `TrackedEntity` directly

### serde
- [tracked-entity-serde](codegen/serde/tracked-entity-serde.md) — OnceLock-aware Serialize/Deserialize; write-once merge; codec integration

---

## cross-cutting
- [error-handling](error-handling/error-handling.md) — error taxonomy, `ErrorCompose`, `OTelEmit`, batch errors, client usage
- [known-issues](known_issues.md) — explicitly deferred design gaps (KI-1 through KI-3)
