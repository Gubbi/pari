# Formal Layer Model

This document is the authoritative architectural reference for Pari's layer model.

The formal layers are:

1. `entity`
2. `workspace`
3. `store`
4. `substrate`
5. `validation`
6. `error`
7. `test`

These names are the canonical architecture vocabulary even where parts of the current design tree still use historical directory names such as `store_layer/`, `workspace_layer/`, or `substrate_layer/`.

## Why This Exists

Pari's design docs have accumulated good detail, but not one explicit statement of the architecture they are describing. This doc fixes that by naming the layers, defining what each one owns, and setting the dependency expectations between them.

The main rule is simple:

- each concept should have one owning layer
- layers may collaborate across explicit boundaries
- no layer should silently absorb another layer's responsibilities

## Layer Definitions

| Layer | Owns | Does not own | May depend on |
|---|---|---|---|
| `entity` | Domain identity, entity definitions, tracked entity representations, shared value types, change-tracking primitives | Actor orchestration, persistence layout, validation policy, caller-facing operation flow | `error` |
| `workspace` | Caller-facing async API, operation handles, generated accessors/setters, request shaping for user intent | In-memory store state, persistence implementation details, validation rule definitions | `entity`, `store`, `validation`, `error` |
| `store` | In-memory entity state, actor/message flow, checkout lifecycle, resolve/load orchestration, persist orchestration, store-owned persistence handoff types | Public caller API ergonomics, persistence layout/encoding, entity rule definitions | `entity`, `substrate`, `validation`, `error` |
| `substrate` | Persistence contracts, schema-driven asset pipeline, backend implementations, storage layout and execution details | Entity Server behavior, caller-facing APIs, validation rule authorship | `entity`, `error`, and explicit store-owned persistence boundary types |
| `validation` | Validation schemas, validation rules, cross-entity validation behavior, validation error details | Persistence, actor flow, caller transport/protocol concerns | `entity`, `error` |
| `error` | Cross-cutting error composition, classification, aggregation, emission, umbrella error types | Domain entities, runtime orchestration, persistence behavior, test logic | none |
| `test` | Verification strategy, test fixtures, integration/end-to-end expectations, test-only support code | Production runtime behavior or ownership decisions | any production layer |

## Dependency Expectations

Pari is not a single straight stack. The runtime usually composes like this:

`workspace -> store -> substrate`

Supporting layers interact with that flow like this:

- `entity` supplies the shared domain vocabulary used by the runtime layers.
- `validation` evaluates entity and workflow correctness where the runtime needs it.
- `error` is cross-cutting infrastructure used by all production layers.
- `test` sits outside production and may exercise every other layer.

More specific expectations:

- `entity` is foundational. Higher layers may use entity-layer types, but entity code must stay free of store, substrate, workspace, and test concerns.
- `workspace` is the caller-facing boundary. It may coordinate store operations and validation-triggered behavior, but it should not own store internals or substrate mechanics.
- `store` is the orchestration boundary between caller intent and persistence. It may coordinate validation and substrate work, but it should not absorb caller-facing API design from `workspace` or persistence implementation concerns from `substrate`.
- `substrate` owns storage concerns. It may consume explicit persistence handoff types defined by the `store` layer, but it should not depend on entity server internals or workspace behavior.
- `validation` owns rule definition and validation-time interpretation, not runtime orchestration.
- `error` stays reusable and cross-cutting; it should not become a back door for moving business logic between layers.
- `test` may reach across layers for verification, but production layers must not depend on test code.

## Composition and Ownership Rules

Use these rules when deciding where a concept belongs:

1. If it defines what an entity is, how it is identified, or how tracked fields behave, it belongs to `entity`.
2. If it defines how callers interact with entities asynchronously, it belongs to `workspace`.
3. If it defines how entities are cached, checked out, resolved, loaded, merged, or persisted in memory, it belongs to `store`.
4. If it defines how data is located, encoded, decoded, or written to durable storage, it belongs to `substrate`.
5. If it defines what counts as valid and how invalid states are reported, it belongs to `validation`.
6. If it defines how failures are classified, composed, aggregated, or emitted, it belongs to `error`.
7. If it exists only to verify behavior, it belongs to `test`.

When a concept touches more than one layer, the owning layer is the one that defines the behavior; other layers should depend on that behavior through an explicit boundary rather than duplicate the logic.

## Within-Layer Structure

Each layer follows a consistent internal split between pure and orchestration components.

### Pure components (`lib/`)

Pure components live in `lib/` subdirectories within each layer. They are responsible
for data transformation, type definitions, encoding/decoding, and rule evaluation.
Every Result-returning function in a `lib/` component emits only `PrimitiveError`.
Pure components have no knowledge of cross-layer concerns.

### Orchestration components (layer root)

Orchestration components live at the layer root. They coordinate across pure components
and adjacent layers. At cross-layer boundaries, orchestration components emit activity
errors — wrapping `PrimitiveError`s received from pure components into the appropriate
activity error type, and forwarding activity errors from deeper layers unchanged.

### Error type by component role

| Component role | Error type at boundaries |
|---|---|
| Pure (`lib/`) | `PrimitiveError` — emitted at the exact point of failure |
| Orchestration | Activity error via `#[activity_error]` — wrap or forward |

`entity` is the sole exception: it has no orchestration layer of its own and stays
with `PrimitiveError` at all boundaries.

### `mod.rs` files

`mod.rs` files contain only `mod` declarations and `pub use` re-exports — no logic,
no `impl` blocks, no free functions. All logic lives in named source files.

---

## Design Tree Mapping

The design tree is being aligned to the formal layer model. The current top-level design directories map like this:

| Current docs area | Formal layer meaning |
|---|---|
| `entity_layer/` | `entity` |
| `workspace_layer/` | `workspace` |
| `store_layer/` | `store` |
| `substrate_layer/` | `substrate` |
| `validation_layer/` | `validation` |
| `error_layer/` | `error` |
| `test_layer/` | `test` |

`codegen/` is intentionally not a formal architecture layer. Code generation belongs to whichever formal layer owns the behavior being generated. Later cleanup should use this rule instead of treating codegen as a separate architectural home.
