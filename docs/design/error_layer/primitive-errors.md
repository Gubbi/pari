# primitive-errors

**Cross-Cutting**

---

## Purpose

This document defines the intended design for Pari's Primitive Errors.

It refines the Primitive layer described in [error-handling](error-handling.md) and focuses on one goal:

- make all Primitive Errors uniform without forcing every Primitive Error struct to manually repeat the same boilerplate fields, constructor logic, trait derives, and observability setup

---

## Design Direction

Primitive Errors should not rely on a hand-written per-error trait implementation.

Instead, Pari should provide a single derive-driven macro contract, along the lines of:

```rust
#[derive(PariError)]
#[error_layer = "primitive"]
```

The exact macro name and attribute syntax can evolve, but the design intent is:

- one declarative entry point for Primitive Errors
- common diagnostic fields provided automatically
- constructor behavior provided automatically
- `thiserror` / `Display` support standardized
- OTel behavior standardized

If `#[error_type = "..."]` is omitted, the macro should derive a default value by converting the Rust type name from `CamelCase` to `snake_case`.

Examples:

- `MalformedFrontmatter` -> `malformed_frontmatter`
- `RenameFailed` -> `rename_failed`

`#[error_type = "..."]` remains available as an override when a specific emitted value is desired.

---

## Mandatory Common Diagnostics

Every Primitive Error must carry the same fixed set of diagnostic details:

- `message`
- `location`
- `span_trace`
- `backtrace`

These are not optional design choices per Primitive Error. They are part of the Primitive layer contract.

### Meaning of each diagnostic

- `message`
  Human-readable explanation of the concrete leaf failure.
- `location`
  The most relevant concrete location for the failure.
  By default this is the error creation site captured by the generated constructor.
- `span_trace`
  Tracing context captured at the point the Primitive Error is created.
- `backtrace`
  Backtrace captured at the point the Primitive Error is created.

---

## Constructor Contract

Primitive Errors should not manually declare and populate these common diagnostic fields in each error struct constructor.

The derive-driven Primitive Error contract should provide an auto-capturing constructor pattern so that:

- `span_trace` is always captured at construction time
- `backtrace` is always captured at construction time
- `location` is auto-captured through a standardized constructor contract
- `message` is accepted through a standardized constructor contract
- the error author only supplies the error-specific details for that Primitive Error

Illustrative direction:

```rust
#[derive(PariError)]
#[error_layer = "primitive"]
#[error_type = "malformed_frontmatter"]
pub struct MalformedFrontmatter {
    pub line: usize,
    pub raw_snippet: String,
}
```

And then the macro-generated constructor shape would conceptually behave like:

```rust
impl MalformedFrontmatter {
    pub fn new(message: impl Into<String>, line: usize, raw_snippet: String) -> Self {
        // message stored
        // location auto-captured
        // span_trace auto-captured
        // backtrace auto-captured
        // detail fields stored
    }
}
```

The exact constructor signature may vary by implementation, but the capture semantics should be fixed.

### Auto-captured location

The intended default is that `location` is auto-captured, not manually passed for every Primitive Error construction.

That auto-captured location represents the error creation site by default.

### Location capture mechanics

The generated constructor captures `location` directly using a standardized caller-location mechanism such as `#[track_caller]` with `std::panic::Location::caller()`.

### Location override

Override is supported for the cases where the creation site is not the most useful domain location.

The design is:

- default constructor: auto-captures location
- explicit override constructor: accepts a caller-supplied location

Illustrative shape:

```rust
impl MalformedFrontmatter {
    pub fn new(message: impl Into<String>, line: usize, raw_snippet: String) -> Self;
    pub fn new_with_location(
        location: ErrorLocation,
        message: impl Into<String>,
        line: usize,
        raw_snippet: String,
    ) -> Self;
}
```

Creation-site location means the source-code location where the Primitive Error is constructed.

Examples where override is useful:

- a parser detects an error while iterating through a decoded document and wants the location to point to the offending document line and column, not the helper function that creates the error
- a repository loader detects a problem in a referenced asset and wants the location to point to that asset path rather than the generic loader call site
- a validator builds a Primitive Error from precomputed source metadata and wants to preserve that source metadata as the error location

Override should be done through `new_with_location(...)`, not by directly setting a managed common field.

---

## What Primitive Error Authors Should Declare

When defining a Primitive Error, authors should only need to declare:

- the error-specific detail fields
- the error type / observability annotation data
- optionally the desired display template if the macro does not infer one

They should not need to repeatedly declare:

- `message`
- `location`
- `span_trace`
- `backtrace`
- standard derive stack for error formatting and emission

### Primitive detail fields

Fields like `line` and `raw_snippet` are the Primitive Error's structured detail fields.

They are:

- not part of the fixed common diagnostics
- specific to that Primitive Error type
- expected to be available to both tracing and error logging through the same derive-driven emission path

They should be treated as structured detail fields by default unless explicitly marked otherwise.

---

## Boilerplate The Macro Should Standardize

For Primitive Errors, the catch-all derive should standardize all of the following:

- `thiserror::Error`
- `Display`
- common diagnostic storage
- constructor-time location capture
- constructor-time `SpanTrace` capture
- constructor-time `Backtrace` capture
- OTel integration
- generated helper/accessor methods used by higher layers and emitters

### Required generated helpers / accessors

The Primitive Error derive should generate the following helpers/accessors:

- `fn error_layer(&self) -> ErrorLayer`
  Always returns `ErrorLayer::Primitive`.
- `fn error_type(&self) -> &'static str`
  Returns the explicit `error_type` or the default snake_case type-name-derived value.
- `fn message(&self) -> &str`
- `fn location(&self) -> &ErrorLocation`
- `fn span_trace(&self) -> &SpanTrace`
- `fn backtrace(&self) -> &Backtrace`
- `fn details(&self) -> &[PrimitiveDetail]`
  Exposes the primitive-specific structured detail fields in emitted form.
- `fn emit(&self)`
  Provided through the standardized `OTelEmit` integration.

### Required generated constructors

The Primitive Error derive should generate:

- `fn new(...) -> Self`
  Uses auto-captured location.
- `fn new_with_location(location: ErrorLocation, ...) -> Self`
  Uses explicit override location.

---

## OTel / Observability Contract

Primitive Errors are the origin point for the most concrete diagnostic evidence in the chain.

The derive-driven Primitive Error contract should therefore also standardize observability requirements:

- Primitive Errors should carry a stable error type identifier for OTel emission.
- `message`, `location`, `span_trace`, and `backtrace` should be consistently available to the OTel emission path.
- error-specific detail fields should also be available for structured emission.
- Primitive Errors should not require hand-written `emit()` implementations in normal cases.

### How detail fields should flow into emission

Primitive detail fields such as `line` and `raw_snippet` are serialized into structured observability fields by the same derive-driven machinery.

### Field-name standardization

Field naming should be finalized as follows:

- Use `opentelemetry_semantic_conventions` wherever a standard field exists.
- For the common exception fields:
  - `error_type` -> OTel exception type
  - `message` -> OTel exception message
  - `backtrace` -> OTel exception stacktrace
- For captured source location:
  - `location.file` -> OTel code file path field
  - `location.line` -> OTel code line number field
  - `location.column` -> OTel code column number field when available
- For primitive-specific detail fields:
  - emit them under the namespaced pattern `error.<error_type>.<snake_case_field_name>`

Examples:

- `line` on `MalformedFrontmatter` -> `error.malformed_frontmatter.line`
- `raw_snippet` on `MalformedFrontmatter` -> `error.malformed_frontmatter.raw_snippet`
- `from` on `RenameFailed` -> `error.rename_failed.from`
- `to` on `RenameFailed` -> `error.rename_failed.to`

This keeps:

- standard fields mapped to OTel semantic conventions
- primitive-specific details stable and machine-readable
- emission deterministic across tracing and logging paths

### Composition with higher-layer fields

Error-specific structured fields are namespaced by concrete `error_type`, not by layer.

The composition rule is:

- standard OTel semantic-convention fields remain at their standard names
- shared semantic fields such as `error.component` and `error.hint` keep their shared names
- every error-specific structured field uses the namespace:
  - `error.<error_type>.<field_name>`

Examples in one emitted chain:

- shared field: `error.component`
- shared field: `error.hint`
- Primitive field: `error.rename_failed.from`
- Primitive field: `error.rename_failed.to`
- Activity field: `error.bad_definition.definition_kind`
- Intermediary field: `error.cross_reference_check_failed.entity_ref`

This keeps higher-layer context and lower-layer evidence from colliding while allowing multiple higher-layer errors to contribute their own structured fields in the same emitted chain.

---

## Relationship to Activity Errors

Primitive Errors carry the concrete leaf failure evidence.

They do **not** carry the higher-level contextual meaning of the subsystem activity. That belongs to the Activity layer.

So the split remains:

- Primitive Error
  Carries concrete failure diagnostics and primitive-specific detail fields.
- Activity Error
  Communicates the outcome, classification (`fix`, `recoverability`), component identity, and recovery hint.

This separation is important because it keeps the Primitive layer focused on leaf evidence while the Activity layer carries the semantic interpretation of that evidence.

---

## Primitive Error Shape

Conceptually, every Primitive Error instance should behave as if it has this shape:

```rust
struct PrimitiveErrorShape<D> {
    message: String,
    location: ErrorLocation,
    span_trace: SpanTrace,
    backtrace: Backtrace,
    details: D,
}
```

This is a conceptual shape, not necessarily the literal implementation shape.

The important part is:

- the common diagnostics are fixed and universal
- the detail payload varies by concrete Primitive Error type

---

## Example

Illustrative direction:

```rust
#[derive(PariError)]
#[error_layer = "primitive"]
#[error_display = "rename failed: {from} -> {to}: {message}"]
pub struct RenameFailed {
    pub from: String,
    pub to: String,
}
```

Conceptually, this definition should be enough for the macro to provide:

- `thiserror::Error`
- `Display`
- stored `message`
- auto-captured `location`
- captured `span_trace`
- captured `backtrace`
- structured OTel emission

without redefining those common fields inside the struct body.

---

## `emit()` Invocation

Primitive Errors should still derive whatever is needed for structured emission, but the normal call site for `emit()` is the top-level Job Error.

So the intended runtime model is:

- Primitive Errors participate in the OTel emission contract
- Activity and Intermediary errors delegate
- `emit()` is normally invoked at the Job layer
- the cascade then reaches the Primitive Error and includes its common diagnostics and detail fields

So yes: in normal usage, `emit()` is called at the Job layer, while Primitive Errors still need standardized emission support so they can contribute their fields during that cascade.

The mechanics are:

- every layer derives or implements `OTelEmit`
- Job-layer `emit()` starts the cascade
- each delegating layer forwards emission through its `source`
- the Primitive layer contributes:
  - OTel exception type
  - OTel exception message
  - OTel exception stacktrace
  - captured location fields
  - all primitive detail fields

This keeps the call site simple while preserving full leaf-level structured evidence.

---

## Leaf Operation Catalog

This section intentionally does **not** catalog the project's currently implemented error enums.

Instead it catalogs:

- the leaf operations each formal layer owns in the current source tree
- the concrete failure origins those operations should be expected to surface

The goal is to design Primitive Errors from operation semantics rather than from today's incidental error shapes.

### How to read this catalog

- "Leaf operation" here means a concrete operation owned by one layer, not a broad user story.
- "Failure origins" means the lowest meaningful failure that the operation can hit, even if current code does not yet model it as a dedicated Primitive Error.
- The same primitive origin may be reused by several operations.
- The `error` layer is included for completeness, but most of its operations should remain operationally infallible.

### `entity` layer

The `entity` layer owns identity, tracked-field state, and entity serialization / deserialization glue generated by `#[derive(Entity)]`.

- Operation: `EntityRef::new(id)`
  - Failure
    - Scenario: invalid top-level identifier format
      Description: the provided `id` does not satisfy the canonical top-level entity identifier contract.
    - Scenario: reserved identifier value
      Description: the provided `id` is syntactically valid but belongs to a reserved namespace or protected value set.
    - Scenario: identifier normalization / canonicalization failure
      Description: the identifier cannot be transformed into the canonical stored form without ambiguity or loss.
- Operation: `EntityRef::with_parent(id, parent)`
  - Failure
    - Scenario: invalid embedded identifier format
      Description: the embedded entity identifier is malformed for the child entity kind.
    - Scenario: parent / child kind mismatch
      Description: the supplied parent identity is not valid for the requested child entity kind.
    - Scenario: missing required parent identity component
      Description: the supplied parent hierarchy omits data required for semantic identity.
    - Scenario: recursive or impossible parent chain
      Description: the parent identity graph describes a cycle or a structurally impossible nesting.
- Operation: `EntityRef` deserialization
  - Failure
    - Scenario: missing required ref fields
      Description: the serialized payload omits one or more fields required to reconstruct entity identity.
    - Scenario: unknown entity kind tag
      Description: the payload identifies an entity kind that Pari does not recognize.
    - Scenario: malformed parent payload
      Description: the serialized parent identity is present but cannot be parsed into a valid parent structure.
    - Scenario: parent kind mismatch
      Description: the payload combines a valid child kind with an incompatible parent kind.
    - Scenario: identifier payload of wrong type
      Description: the identity field exists but uses the wrong scalar or structured representation.
    - Scenario: duplicate / conflicting fields in incoming payload
      Description: the serialized form contains overlapping or contradictory identity data.
- Operation: `ParentKind::deserialize_parent(...)`
  - Failure
    - Scenario: missing required parent object
      Description: an embedded entity payload omits parent identity even though parentage is mandatory.
    - Scenario: unexpected parent on top-level entity
      Description: a top-level entity payload incorrectly includes parent identity data.
    - Scenario: malformed nested parent ref
      Description: the nested parent reference exists but is structurally invalid.
    - Scenario: parent entity kind mismatch
      Description: the nested parent reference resolves to the wrong entity kind for the child.
- Operation: generated tracked-entity `from_json_value(...)`
  - Failure
    - Scenario: malformed entity payload
      Description: the input shape cannot be interpreted as the target tracked entity at all.
    - Scenario: missing required field
      Description: a required domain field is absent from the incoming payload.
    - Scenario: wrong JSON type for field
      Description: a field is present but encoded as an incompatible JSON type.
    - Scenario: unknown enum discriminant
      Description: an enum-valued field contains a tag or discriminant the entity model does not support.
    - Scenario: invalid nested entity ref
      Description: a nested reference field fails its identity contract during reconstruction.
- Operation: generated tracked-entity `to_json_value(...)`
  - Failure
    - Scenario: by design should stay infallible for a valid tracked entity
      Description: if this fails, it indicates drift between the tracked-entity model and its declared serialization contract rather than an ordinary caller-facing data error.
- Operation: `TrackedField::initialize(value)`
  - Failure
    - Scenario: write-once field already initialized from a different source
      Description: initialization is attempted after the field has already been populated through another authoritative path.
- Operation: `TrackedField::mutated(value)` / setter-side replacement
  - Failure
    - Scenario: by design should stay infallible for type-correct replacement
      Description: semantic invalidity should be rejected by validation and setter workflows, not by the `TrackedField::mutated(...)` helper itself.
- Operation: `TrackedField::get()` as used by generated accessors
  - Failure
    - Scenario: unloaded field reached through an internal caller path that bypassed the accessor load contract
      Description: caller-facing accessors should transparently load before reading, so this should only surface if internal code bypasses that contract.
- Operation: `TrackedField::reset_dirty()`
  - Failure
    - Scenario: by design should stay infallible as an internal bookkeeping operation
      Description: dirty reset should be a deterministic state transition once a valid tracked field exists.

### `workspace` layer

The `workspace` layer owns caller-facing async operations and generated accessors / setters.

- Operation: `EntityClient::resolve(any_ref)`
  - Failure
    - Scenario: request transport unavailable
      Description: the workspace cannot deliver the resolve request to the store actor boundary.
    - Scenario: malformed request payload
      Description: the request shape reaching the boundary is invalid for the operation contract.
    - Scenario: store actor terminated
      Description: the backing actor shuts down or disappears before the request completes.
    - Scenario: entity missing from durable backing store
      Description: the requested identity does not exist in persistence and cannot be resolved into a tracked stub.
    - Scenario: existence check infrastructure failure
      Description: the substrate fails while checking whether the referenced entity exists.
- Operation: `EntityClient::insert(entity)`
  - Failure
    - Scenario: request transport unavailable
      Description: the insert request cannot cross the workspace-to-store boundary.
    - Scenario: incoming entity violates structural / semantic / cross-entity rules
      Description: the provided tracked entity fails validation required for insertion.
    - Scenario: entity kind unsupported by backing substrate
      Description: the current substrate cannot persist or reason about the entity kind being inserted.
    - Scenario: substrate preflight failure while resolving cross-entity dependencies
      Description: validation or reference checks require substrate data that cannot be retrieved successfully.
- Operation: `EntityClient::remove(any_ref)`
  - Failure
    - Scenario: request transport unavailable
      Description: the remove request cannot be delivered to the store.
    - Scenario: remove requested for unknown entity
      Description: the workspace asks to remove an entity the store does not know about.
    - Scenario: remove blocked by checkout state
      Description: removal is attempted while the entity is in a checkout state that forbids it.
    - Scenario: actor lifecycle failure while processing request
      Description: the remove operation loses its actor execution context before producing a stable outcome.
- Operation: `EntityClient::checkout(any_ref)`
  - Failure
    - Scenario: request transport unavailable
      Description: the checkout request cannot cross the actor boundary.
    - Scenario: entity unknown in current workspace state
      Description: the requested entity is not present in the store cache at checkout time.
    - Scenario: checkout blocked because entity already checked out
      Description: another checkout already owns the mutable working state for this entity.
    - Scenario: stale caller view of entity identity
      Description: the caller is attempting to checkout an entity identity no longer valid in the current workspace session.
- Operation: `EntityClient::load(any_ref, field)`
  - Failure
    - Scenario: request transport unavailable
      Description: the load request cannot reach the store actor.
    - Scenario: target entity unresolved in store state
      Description: the store has no tracked entry corresponding to the requested identity.
    - Scenario: unknown field name
      Description: the requested field is not part of the entity's schema-backed load surface.
    - Scenario: invalid load strategy
      Description: the substrate cannot derive a coherent load strategy for the requested field.
    - Scenario: resolver path expansion failure
      Description: the substrate cannot convert the entity identity plus schema into a concrete asset location.
    - Scenario: executor read failure
      Description: the underlying asset read fails at the storage execution boundary.
    - Scenario: codec decode failure
      Description: the fetched asset cannot be decoded into the requested field payload.
    - Scenario: loaded payload fails validation
      Description: the persisted data decodes but violates validation rules when reintroduced as tracked state.
- Operation: `EntityClient::ensure_mutable(any_ref, field)`
  - Failure
    - Scenario: request transport unavailable
      Description: the mutability preflight request cannot reach the store actor.
    - Scenario: target entity unresolved in store state
      Description: the store does not have a tracked entity for the requested identity.
    - Scenario: unknown field name
      Description: the requested field has no defined mutability / load strategy.
    - Scenario: prerequisite field load failure
      Description: one of the fields that must be loaded first cannot be loaded successfully.
    - Scenario: target field load blocked by substrate or validation failure
      Description: the field itself cannot be materialized into a mutable state because a lower boundary fails.
    - Scenario: field cannot be made mutable under current load strategy
      Description: the computed schema contract says the field is not safely mutable in the current state.
- Operation: `EntityClient::persist()`
  - Failure
    - Scenario: request transport unavailable
      Description: the persist request cannot cross the workspace-to-store boundary.
    - Scenario: persist blocked by outstanding checkouts
      Description: the store still has checked-out entities that must be reconciled before persistence.
    - Scenario: entity-to-asset mapping failure
      Description: the substrate cannot derive the asset set required to persist the pending changes.
    - Scenario: serialization / encoding failure
      Description: tracked entities cannot be converted into encoded substrate payloads.
    - Scenario: executor write failure
      Description: one or more storage writes fail at the execution boundary.
    - Scenario: partial write batch failure
      Description: the persist batch succeeds for some assets and fails for others.
    - Scenario: delete failure during remove persist
      Description: asset deletion for removed entities fails while applying the persist set.
- Operation: `EntityClient::undo_commit(any_ref)`
  - Failure
    - Scenario: request transport unavailable
      Description: the undo-commit request cannot reach the store actor.
    - Scenario: entity not in an undoable state
      Description: the target entity is neither newly added nor modified in a way that supports rollback.
    - Scenario: entity currently checked out
      Description: rollback is attempted while mutable checkout ownership is still active.
    - Scenario: actor state transition invariant violation
      Description: the store cannot reconcile the undo request with its lifecycle state model.
- Operation: `EntityClient::unload(any_ref)`
  - Failure
    - Scenario: request transport unavailable
      Description: the unload request cannot cross the actor boundary.
    - Scenario: entity unknown to store
      Description: there is no tracked entity entry to unload.
    - Scenario: entity currently checked out
      Description: unload is attempted while mutable checkout ownership is still active.
    - Scenario: entity has unpersisted additions or modifications
      Description: unload would discard dirty in-memory state that has not been persisted or rolled back.
    - Scenario: actor state transition invariant violation
      Description: the store cannot apply the unload request without violating lifecycle rules.
- Operation: generated tracked method `TrackedEntity::commit()`
  - Failure
    - Scenario: request transport unavailable
      Description: the commit request cannot reach the store actor.
    - Scenario: commit blocked by validation failure
      Description: the candidate tracked entity violates required validation rules.
    - Scenario: commit blocked by unresolved cross-entity references
      Description: the candidate state refers to entities or relationships that do not resolve consistently.
    - Scenario: internal merge / dirty-state invariant failure
      Description: the checked-out state cannot be merged back into store state without violating bookkeeping invariants.
- Operation: generated tracked method `TrackedEntity::undo_checkout()`
  - Failure
    - Scenario: request transport unavailable
      Description: the undo-checkout request cannot be delivered to the store.
    - Scenario: entity was not checked out
      Description: the caller attempts to end a checkout that does not exist.
    - Scenario: checkout lifecycle invariant violation
      Description: the checkout lifecycle state is internally inconsistent with the requested operation.
- Operation: generated async field accessor
  - Failure
    - Scenario: underlying field load failure
      Description: the implicit load required by the accessor fails before the field can be read.
    - Scenario: field remains unavailable after successful load path
      Description: the load path appears to complete but the field is still not present in tracked state.
    - Scenario: tracked-field state mismatch after load
      Description: loaded data and accessor expectations disagree about the field's resulting state.
- Operation: generated setter
  - Failure
    - Scenario: `ensure_mutable` failure
      Description: the setter cannot obtain the preconditions required to perform a safe mutation.
    - Scenario: candidate mutation violates structural rules
      Description: the new value breaks structural invariants for the target field.
    - Scenario: candidate mutation violates semantic rules
      Description: the new value is structurally valid but semantically invalid in context.
    - Scenario: setter applied to wrong tracked entity kind
      Description: generated setter code is invoked against an incompatible tracked type or dispatch path.
    - Scenario: post-validation state swap invariant failure
      Description: the candidate state validates but cannot be safely installed into the tracked entity instance.

### `store` layer

The `store` layer owns the actor protocol, in-memory entity state, checkout lifecycle, load orchestration, and persist orchestration.

- Operation: `EntityServer::init(substrate)`
  - Failure
    - Scenario: actor spawn failure
      Description: the runtime cannot create or schedule the backing store actor.
    - Scenario: double initialization
      Description: global store actor initialization is attempted more than once.
    - Scenario: substrate boot / constructor failure if boot becomes fallible before actor start
      Description: the actor cannot be started because the chosen substrate fails its own initialization contract.
- Operation: `EntityServer::request(request)`
  - Failure
    - Scenario: request channel send failure
      Description: the actor request channel rejects or loses the outbound request.
    - Scenario: reply channel dropped
      Description: the actor never delivers a reply on the return channel.
    - Scenario: actor terminated mid-request
      Description: the actor disappears after accepting the request but before completing it.
    - Scenario: request / response protocol mismatch
      Description: the actor boundary produces a response shape inconsistent with the submitted request.
- Operation: `Store::handle(request)`
  - Failure
    - Scenario: invalid request / response pairing
      Description: dispatch selects a response variant that does not match the incoming operation.
    - Scenario: internal dispatch bug causing wrong response variant
      Description: store routing chooses the wrong operation branch or response type.
    - Scenario: actor state corruption preventing dispatch
      Description: store state cannot be inspected or mutated safely enough to dispatch the request.
- Operation: `Store::resolve(any_ref)`
  - Failure
    - Scenario: entity absent from cache and not present in substrate
      Description: the entity cannot be found either in memory or in durable backing storage.
    - Scenario: substrate exists check failure
      Description: the store cannot determine whether the entity exists in persistence.
    - Scenario: failed stub creation
      Description: the store cannot construct the in-memory stub representation needed for later loading.
    - Scenario: cached entity entry corrupted
      Description: the cache contains an entry for the identity but its state is internally invalid.
- Operation: `Store::insert(entity)`
  - Failure
    - Scenario: commit-time validation failure for new entity
      Description: the new entity fails validation before being admitted into store state.
    - Scenario: duplicate identity collision with incompatible cached state
      Description: the store already contains the same identity with conflicting tracked state.
    - Scenario: remove-then-reinsert reconciliation failure
      Description: reintroducing a previously removed identity cannot be reconciled with the change sets.
    - Scenario: dirty-state bookkeeping invariant failure
      Description: the store cannot place the entity into added / modified / removed bookkeeping consistently.
- Operation: `Store::checkout(any_ref)`
  - Failure
    - Scenario: entity already checked out
      Description: the store already has mutable checkout ownership recorded for the entity.
    - Scenario: entity absent from store cache
      Description: checkout is attempted before the entity has been resolved into store state.
    - Scenario: checkout set bookkeeping corruption
      Description: the checkout tracking set cannot represent the requested lifecycle transition coherently.
- Operation: `Store::commit(entity)`
  - Failure
    - Scenario: validation failure for added entity
      Description: a newly inserted entity fails full validation at commit time.
    - Scenario: cross-entity validation failure for dirty fields
      Description: a modified entity breaks cross-entity rules when trying to commit dirty changes.
    - Scenario: checked-out set reconciliation failure
      Description: the entity cannot be removed from checkout ownership consistently during commit finalization.
    - Scenario: merge-dirty-into failure
      Description: the checked-out snapshot cannot be merged back into the cached entity safely.
    - Scenario: cached entity missing during commit finalization
      Description: the commit reaches merge time without a stable target entity entry in the cache.
    - Scenario: dirty-state reset failure
      Description: dirty flags cannot be normalized after a successful commit transition.
- Operation: `Store::remove_entity(any_ref)`
  - Failure
    - Scenario: remove requested for missing cached entity
      Description: the store has no in-memory entity entry matching the requested identity.
    - Scenario: remove blocked by checkout state
      Description: the entity is currently checked out and cannot be removed safely.
    - Scenario: added / modified / removed set bookkeeping corruption
      Description: store change-set bookkeeping cannot absorb the remove transition consistently.
- Operation: `Store::persist()`
  - Failure
    - Scenario: pending checkout precondition failure
      Description: the store refuses to persist while mutable checkout ownership is still open.
    - Scenario: change-set construction failure
      Description: the store cannot derive a coherent persist set from its in-memory tracking structures.
    - Scenario: substrate persist batch failure
      Description: one or more substrate operations fail while applying the persist set.
    - Scenario: post-persist dirty reset failure
      Description: store dirty flags cannot be normalized after a successful substrate persist.
    - Scenario: add / modify / remove set cleanup failure
      Description: the store cannot clear or reconcile its change tracking after persist completion.
- Operation: `Store::ensure_persistable()`
  - Failure
    - Scenario: outstanding checkout count nonzero
      Description: the persist precondition check fails because entities are still checked out.
    - Scenario: checkout bookkeeping inconsistent with persist expectations
      Description: checkout state metadata is internally inconsistent during persist preflight.
- Operation: `Store::load_field(any_ref, field)`
  - Failure
    - Scenario: same primitive origins as `load_fields`
      Description: single-field load is a specialized wrapper over the broader multi-field load orchestration.
    - Scenario: single-field request routed to invalid field selection
      Description: the one-field wrapper forwards a field selection that is invalid for the entity schema.
- Operation: `Store::ensure_mutable(any_ref, field)`
  - Failure
    - Scenario: unknown field in load strategy lookup
      Description: the schema cannot describe load / mutation behavior for the requested field.
    - Scenario: prerequisite discovery failure
      Description: the substrate cannot derive the prerequisite field set needed for safe mutation.
    - Scenario: prerequisite load failure
      Description: one of the prerequisite fields cannot be loaded successfully.
    - Scenario: target field load failure
      Description: the target field itself cannot be loaded into mutable state.
    - Scenario: mutable-without-load contract miscomputed for asset mapping
      Description: the computed mutability contract disagrees with the actual asset dependency shape.
- Operation: `Store::load_fields(any_ref, fields, include_prerequisites)`
  - Failure
    - Scenario: entity missing from store state
      Description: load orchestration begins without a tracked entity entry in the cache.
    - Scenario: unknown field in schema lookup
      Description: one of the requested fields does not exist in the schema-backed load surface.
    - Scenario: recursive prerequisite cycle
      Description: prerequisite expansion leads to a cycle rather than a finite load plan.
    - Scenario: substrate load failure
      Description: the substrate cannot fetch or decode the requested asset data.
    - Scenario: decoded payload cannot merge into tracked entity
      Description: the loaded field map cannot be applied cleanly to the current tracked entity state.
    - Scenario: loaded payload fails validation
      Description: the loaded data decodes successfully but violates validation rules.
    - Scenario: unresolved referenced entity cannot be stubbed
      Description: referenced identities discovered during load cannot be materialized into store stubs.
    - Scenario: cache replacement / initialization invariant failure
      Description: the store cannot initialize or replace cached state consistently after load.
- Operation: `Store::undo_checkout(any_ref)`
  - Failure
    - Scenario: entity not currently checked out
      Description: the store cannot undo a checkout that is not active.
    - Scenario: checkout bookkeeping corruption
      Description: checkout state metadata is inconsistent with the requested undo transition.
- Operation: `Store::undo_commit(any_ref)`
  - Failure
    - Scenario: entity not in undoable added / modified state
      Description: the entity has no tracked commit transition to roll back.
    - Scenario: entity still checked out
      Description: rollback is attempted while mutable checkout ownership still exists.
    - Scenario: rollback stub creation failure
      Description: the store cannot recreate the stub state needed for rollback.
    - Scenario: modified set bookkeeping corruption
      Description: rollback cannot be reconciled with the store's modified-entity tracking.
- Operation: `Store::unload(any_ref)`
  - Failure
    - Scenario: entity unknown
      Description: there is no tracked entity entry to unload.
    - Scenario: entity checked out
      Description: unload is attempted while mutable checkout ownership remains active.
    - Scenario: entity has unpersisted changes
      Description: unload would discard dirty in-memory state.
    - Scenario: stub replacement failure
      Description: the store cannot replace the loaded entity with a stable stub representation.
- Operation: `Store::validate_committed_entity(...)`
  - Failure
    - Scenario: structural validation failure
      Description: the entity violates one or more structural rules during commit preflight.
    - Scenario: semantic validation failure
      Description: the entity violates one or more semantic rules during commit preflight.
    - Scenario: cross-entity validation failure
      Description: the entity violates one or more cross-entity rules during commit preflight.
    - Scenario: validation dispatch mismatch for tracked entity kind
      Description: validation is invoked with a tracked wrapper / schema combination that does not line up.
- Operation: `Store::validate_loaded_entity(...)`
  - Failure
    - Scenario: structural validation failure from persisted payload
      Description: persisted data breaks structural rules when loaded back into tracked form.
    - Scenario: semantic validation failure from persisted payload
      Description: persisted data is structurally valid but semantically invalid in context.
    - Scenario: cross-entity validation failure from persisted payload
      Description: loaded data breaks cross-entity rules against the wider graph.
    - Scenario: validation dispatch mismatch for tracked entity kind
      Description: the store validates loaded data against the wrong tracked type or schema.

### `substrate` layer

The `substrate` layer owns schema-driven persistence contracts and concrete backend mechanics.

- Operation: `Substrate::load_strategy(entity_kind, field)`
  - Failure
    - Scenario: unknown field in schema
      Description: the schema registry does not know how to load the requested field.
    - Scenario: duplicated or ambiguous field mapping
      Description: more than one schema asset appears to own the same field.
    - Scenario: invalid asset dependency graph
      Description: the schema's prerequisite relationships do not describe a coherent load plan.
    - Scenario: cyclic prerequisite chain
      Description: field prerequisites reference one another in a cycle.
    - Scenario: unsupported entity kind in schema registry
      Description: the substrate has no schema entry for the requested entity kind.
- Operation: `Substrate::exists(refs)`
  - Failure
    - Scenario: path resolution failure
      Description: entity references cannot be turned into concrete asset locations.
    - Scenario: batch executor head failure
      Description: the executor cannot perform one or more existence checks.
    - Scenario: response shape mismatch from executor
      Description: executor responses do not line up with the requested existence-check batch.
    - Scenario: ref-to-path serialization failure
      Description: the entity reference cannot be projected into the substrate path template input.
- Operation: `Substrate::load(entity, fields)`
  - Failure
    - Scenario: entity-to-json conversion failure
      Description: the tracked entity cannot be projected into the plain JSON form required for path resolution.
    - Scenario: asset selection failure
      Description: the substrate cannot determine which assets are needed for the requested fields.
    - Scenario: path resolution failure
      Description: selected assets cannot be mapped to concrete locations.
    - Scenario: executor read failure
      Description: one or more asset reads fail at the execution boundary.
    - Scenario: codec decode failure
      Description: fetched asset payloads cannot be decoded into field values.
    - Scenario: decoded field map cannot merge into tracked entity
      Description: decoded data cannot be reapplied as tracked state for the target entity.
    - Scenario: response count / ordering mismatch
      Description: the executor returns a batch shape inconsistent with the selected assets.
- Operation: `Substrate::persist(changes)`
  - Failure
    - Scenario: change payload cannot serialize
      Description: one or more change entries cannot be projected into the serialization boundary.
    - Scenario: asset selection failure
      Description: the substrate cannot determine which assets should be written for the change set.
    - Scenario: path resolution failure
      Description: write targets cannot be resolved into concrete asset locations.
    - Scenario: codec encode failure
      Description: entity state cannot be encoded into the target asset format.
    - Scenario: executor write / patch / post / delete failure
      Description: one or more requested storage operations fail while applying the change set.
    - Scenario: partial batch failure
      Description: the substrate applies only a subset of the requested persistence operations.
    - Scenario: change iterator contains incompatible or impossible state
      Description: the incoming persist set describes a state transition the substrate contract cannot honor.
- Operation: `EntitySchema::lookup(field)`
  - Failure
    - Scenario: unknown schema field
      Description: the requested field does not appear anywhere in the schema.
    - Scenario: duplicate field ownership across assets
      Description: the schema assigns the same field to more than one asset.
    - Scenario: corrupted field-index cache
      Description: cached lookup metadata no longer matches the schema definition.
- Operation: `EntitySchema::load_strategy_for(field)`
  - Failure
    - Scenario: unknown field
      Description: the field cannot be found in schema lookup.
    - Scenario: invalid prerequisite declaration
      Description: prerequisite metadata names fields or assets that do not form a valid plan.
    - Scenario: impossible mutable-without-load classification
      Description: the schema claims a field is mutable without load when its asset shape makes that impossible.
- Operation: `EntitySchema::assets_for_read(fields)`
  - Failure
    - Scenario: unknown requested field
      Description: one or more requested fields do not exist in the schema.
    - Scenario: duplicate / conflicting asset selection
      Description: read asset selection cannot produce a coherent, deduplicated asset set.
    - Scenario: inconsistent schema asset metadata
      Description: the asset metadata needed for read selection is internally contradictory.
- Operation: `EntitySchema::assets_for_write(dirty_fields)`
  - Failure
    - Scenario: unknown dirty field
      Description: the incoming dirty field set refers to a field outside the schema.
    - Scenario: dirty field mapped to unsupported asset kind
      Description: the selected asset kind cannot support the required write behavior for the field.
    - Scenario: inconsistent schema asset metadata
      Description: asset metadata needed for write selection does not form a coherent plan.
- Operation: `AssetMapper::select_for_read / select_for_write`
  - Failure
    - Scenario: schema-selection failure delegated from entity schema
      Description: asset mapping fails because schema lookup or asset selection fails beneath it.
    - Scenario: unsupported partial-write mapping
      Description: a requested partial write cannot be expressed by the selected asset kind.
- Operation: `substrate::serde::entity_to_json(entity)`
  - Failure
    - Scenario: tracked entity cannot serialize to plain JSON
      Description: the tracked entity cannot be projected into the serialization form expected by substrate code.
    - Scenario: invalid nested ref serialization
      Description: a nested entity reference fails identity serialization during projection.
    - Scenario: unloaded-but-required field exposed to persistence
      Description: persistence tries to serialize a field that has not been loaded or initialized.
- Operation: `substrate::serde::merge_field_map_into(target, field_map)`
  - Failure
    - Scenario: decoded field map contains incompatible types
      Description: decoded values do not match the expected tracked field types.
    - Scenario: decoded path collides with non-object path segment
      Description: dot-path reconstruction runs into incompatible intermediate shapes.
    - Scenario: partial payload cannot deserialize into tracked entity
      Description: the reconstructed partial entity payload is not valid for the target tracked entity.
    - Scenario: extension flattening creates conflicting keys
      Description: flattened extension fields collide with explicit entity field paths.
- Operation: `substrate::serde::deserialize_entity_from_value(any_ref, value)`
  - Failure
    - Scenario: entity kind payload mismatch
      Description: the payload cannot be deserialized as the tracked entity kind named by the reference.
    - Scenario: required field missing
      Description: the deserialization payload omits one or more required fields.
    - Scenario: invalid nested entity ref
      Description: a nested reference fails its identity contract during deserialization.
    - Scenario: JSON shape incompatible with tracked entity definition
      Description: the payload shape cannot be mapped onto the target tracked entity.
- Operation: `RepoSubstrate::new(root)`
  - Failure
    - Scenario: root directory creation failure
      Description: the substrate cannot create or validate the target root directory.
    - Scenario: stale cleanup traversal failure
      Description: initialization cannot inspect pre-existing stale directories safely.
    - Scenario: stale cleanup deletion failure
      Description: initialization cannot remove stale substrate artifacts.
    - Scenario: invalid root path
      Description: the configured root path is syntactically or semantically unusable.
- Operation: `cleanup_stale(root)`
  - Failure
    - Scenario: directory read failure
      Description: the cleanup walk cannot read a directory needed for stale inspection.
    - Scenario: directory entry read failure
      Description: one or more directory entries cannot be enumerated or inspected.
    - Scenario: stale directory deletion failure
      Description: a stale directory is identified but cannot be deleted.
    - Scenario: path unexpectedly not traversable
      Description: the cleanup walk encounters a path shape it cannot traverse safely.
- Operation: `RepoLocationResolver::resolve(...)`
  - Failure
    - Scenario: unresolved template placeholder
      Description: the path template references data that is missing from the entity projection.
    - Scenario: missing parent base data
      Description: a parent-derived path component cannot be computed from the available identity data.
    - Scenario: invalid path template
      Description: the configured template is syntactically or semantically invalid.
    - Scenario: path escaping / traversal outside root
      Description: path resolution would produce a location outside the substrate's permitted root.
- Operation: `RepoCodec::encode(entity_json, schema)`
  - Failure
    - Scenario: expected scalar / object / array shape not present
      Description: a field's projected JSON shape does not match the slot's encoding requirements.
    - Scenario: unsupported slot composition
      Description: the selected schema slots cannot be encoded together into a valid asset body.
    - Scenario: frontmatter serialization failure
      Description: the frontmatter portion of the asset cannot be rendered into a valid encoded form.
    - Scenario: section rendering failure
      Description: one or more section payloads cannot be rendered into the target document format.
    - Scenario: impossible mixed-asset mapping
      Description: the schema requests an asset composition that the repo codec does not support.
- Operation: `RepoCodec::decode(raw, schema)`
  - Failure
    - Scenario: malformed frontmatter
      Description: the encoded asset body contains a frontmatter block that cannot be parsed safely.
    - Scenario: invalid YAML
      Description: the frontmatter block exists but is not valid YAML.
    - Scenario: invalid JSON conversion from YAML
      Description: parsed YAML cannot be converted into the JSON shape expected by the entity model.
    - Scenario: unsupported section body shape
      Description: a section body does not match the shape expected by the schema slot.
    - Scenario: duplicate heading collision
      Description: multiple headings or sections collapse into an ambiguous field mapping.
    - Scenario: schema slot cannot be reconstructed from document body
      Description: the asset content does not contain enough valid structure to recover the requested slot.
- Operation: `RepoExecutor::execute(ops)`
  - Failure
    - Scenario: file read failure
      Description: a requested file cannot be read from the filesystem.
    - Scenario: parent directory creation failure
      Description: the executor cannot create directories needed for a write.
    - Scenario: file write failure
      Description: a write target cannot be created or overwritten successfully.
    - Scenario: file delete failure
      Description: a requested file cannot be removed during delete execution.
    - Scenario: unsupported executor operation kind
      Description: the repo executor receives an operation type it does not implement.
    - Scenario: path permission failure
      Description: the underlying filesystem rejects access to the requested path.
- Operation: `InMemoryCodec::encode(...)`
  - Failure
    - Scenario: JSON serialization failure
      Description: selected field data cannot be serialized into the in-memory encoded form.
    - Scenario: field extraction shape mismatch
      Description: a requested field cannot be extracted in a shape the codec can represent.
- Operation: `InMemoryCodec::decode(...)`
  - Failure
    - Scenario: malformed JSON payload
      Description: the stored in-memory payload is not valid JSON.
    - Scenario: payload shape incompatible with expected field map
      Description: the decoded JSON cannot be interpreted as the expected field-value mapping.
- Operation: `InMemoryExecutor::execute(ops)`
  - Failure
    - Scenario: requested asset missing
      Description: a read targets an in-memory asset key that does not exist.
    - Scenario: lock poisoning or shared-state corruption
      Description: the shared in-memory asset store cannot be accessed safely.
    - Scenario: response shape mismatch across batch execution
      Description: execution results do not line up with the requested operation batch.
- Operation: `InMemorySubstrate::new / with_storage`
  - Failure
    - Scenario: inconsistent injected storage handle
      Description: the provided shared storage handle does not satisfy substrate expectations.
    - Scenario: incompatible preloaded asset state
      Description: the initial in-memory asset state cannot support coherent substrate operations.
- Operation: `VoidSubstrate::load(...)`
  - Failure
    - Scenario: intentional unsupported-load sentinel
      Description: the no-op substrate is asked to materialize entity data even though it is intentionally non-loading.
- Operation: `VoidSubstrate::persist(...)`
  - Failure
    - Scenario: by design should stay infallible unless the contract is tightened later
      Description: this operation is currently intended as a no-op and should not invent failure unless the contract changes.
- Operation: `VoidSubstrate::exists(...)`
  - Failure
    - Scenario: by design should stay infallible unless the contract is tightened later
      Description: this operation is currently intended as a deterministic no-op existence check.

### `validation` layer

The `validation` layer owns rule definition, rule execution, and validation error payloads.

- Operation: `run_validations(tracked, fields, kinds)`
  - Failure
    - Scenario: invalid field selection
      Description: validation is asked to evaluate a field that is not represented in the schema.
    - Scenario: tracked entity kind / schema mismatch
      Description: the validation schema and tracked entity do not correspond to the same entity type.
    - Scenario: structural rule violation
      Description: one or more structural rules reject the candidate state.
    - Scenario: semantic rule violation
      Description: one or more semantic rules reject the candidate state.
    - Scenario: cross-entity rule violation
      Description: one or more cross-entity rules reject the candidate state.
    - Scenario: rule dispatch invariant failure
      Description: validation dispatch cannot route the rule set coherently for the tracked entity.
- Operation: `run_validations_for_entity(tracked_entity, fields, kinds)`
  - Failure
    - Scenario: tracked wrapper dispatch failure
      Description: the type-erased tracked wrapper cannot dispatch to the correct validation path.
    - Scenario: wrong tracked subtype extracted for entity kind
      Description: the wrapper resolves to the wrong tracked subtype for the underlying entity kind.
    - Scenario: delegated `run_validations` failure origins
      Description: all lower-level validation-runner failures remain possible through this dispatch wrapper.
- Operation: `ValidationSchema::all_field_names()`
  - Failure
    - Scenario: schema map inconsistency
      Description: the schema's internal rule maps do not describe a coherent field set.
    - Scenario: duplicate / conflicting field names across rule maps
      Description: different rule maps name fields in ways that cannot be reconciled into one field inventory.
- Operation: `build_path(field, sub_path)`
  - Failure
    - Scenario: invalid sub-path format
      Description: a rule reports a nested violation path in a format that cannot be normalized safely.
    - Scenario: field-path concatenation producing impossible output path
      Description: path joining produces a field location string that cannot correspond to a real field path.
- Operation: structural primitive rule helpers such as `kebab_case`, `camel_case`, collection-shape rules
  - Failure
    - Scenario: malformed scalar value
      Description: a scalar input does not match the required primitive format.
    - Scenario: malformed collection value
      Description: a collection input breaks the shape rules for the validated field.
    - Scenario: naming-format violation
      Description: a value breaks the required naming convention or lexical contract.
    - Scenario: duplicate entry violation
      Description: a collection contains entries that must be unique but are not.
    - Scenario: empty-required-value violation
      Description: a field or collection entry is empty where non-empty content is required.
- Operation: semantic workflow / relay / hook / team rules
  - Failure
    - Scenario: workflow graph inconsistency
      Description: the entity's internal dependency or state graph is not semantically coherent.
    - Scenario: illegal dependency reference
      Description: a declared dependency points to a step, task, or relationship that is not semantically valid.
    - Scenario: illegal state transition reference
      Description: a referenced state transition is not allowed by the workflow semantics.
    - Scenario: invalid on-reject target
      Description: rejection-handling configuration points to an invalid or impossible target.
    - Scenario: duplicate semantic relationship
      Description: the same semantic relationship is declared more than once in a conflicting way.
    - Scenario: missing required companion state
      Description: one stateful concept depends on another required state that is absent.
- Operation: cross-entity rule helpers such as `ref_exists`, `all_refs_exist`, `raci_roles_exist`, `hook_call_inputs_valid`
  - Failure
    - Scenario: referenced entity absent
      Description: a required referenced entity does not exist.
    - Scenario: referenced entity of wrong kind
      Description: a reference resolves to an entity kind incompatible with the field semantics.
    - Scenario: reference set incomplete
      Description: the set of related references required for semantic correctness is not complete.
    - Scenario: referenced input / role / hook target inconsistent with definition
      Description: a referenced external definition exists but does not match the consuming entity's expectations.
- Operation: `ValidationErrors::extend(...)`
  - Failure
    - Scenario: aggregation bookkeeping failure
      Description: validation error aggregation cannot preserve a coherent combined error set.
    - Scenario: impossible mix of validation kinds if future contracts require stronger grouping
      Description: aggregation produces a combination of violation kinds that later contracts may choose to forbid.

Validation differs from other layers because many of its leaf failures are domain violations rather than operational faults. Those domain violations are still Primitive-level evidence and should still carry structured fields such as:

- field path
- rule kind
- offending value or reference
- violated constraint identity

### `error` layer

The `error` layer owns composition, classification, aggregation, and emission. Most of these operations should be designed to avoid surfacing new user-visible primitive failures.

- Operation: `Severity::from_classification(...)`
  - Failure
    - Scenario: invalid classification combination only if the type model is broken
      Description: under the intended model this function should stay total and deterministic over valid `FixDomain` and `Recoverability` pairs.
- Operation: `ErrorCompose::fix_domain / recoverability / severity`
  - Failure
    - Scenario: missing compose metadata
      Description: a derived or implemented error type fails to declare the composition information the contract requires.
    - Scenario: invalid delegated source chain
      Description: a delegating error claims composition through a source chain that is incomplete or inconsistent.
    - Scenario: composition contract violated by a derived type
      Description: a type participates in error composition without satisfying the rules needed for stable classification.
- Operation: `dyn ErrorCompose::as_error::<E>()`
  - Failure
    - Scenario: downcast chain broken by incorrect delegation metadata
      Description: the composed error chain cannot expose the expected inner error type because delegation metadata is wrong.
    - Scenario: erased inner error unavailable despite delegating contract
      Description: a delegating wrapper promises an inner error but cannot actually surface it for downcasting.
- Operation: `BatchError::new(errors)`
  - Failure
    - Scenario: empty batch when caller promised at least one failure
      Description: a batch wrapper is created for an operation outcome that did not actually produce any concrete failures.
    - Scenario: heterogeneous batch created from incompatible operation contexts
      Description: one batch is built from failures that do not belong to the same logical operation context.
- Operation: `BatchError` aggregation of classification
  - Failure
    - Scenario: invalid worst-case ordering if classification taxonomy changes without updating aggregation rules
      Description: batch aggregation computes the wrong dominant classification because the ordering rules and taxonomy drift apart.
- Operation: `OTelEmit::emit()`
  - Failure
    - Scenario: observability field extraction failure
      Description: an error cannot expose the structured fields needed for emission.
    - Scenario: invalid structured field naming
      Description: emission tries to use field names that do not satisfy the observability contract.
    - Scenario: exporter / subscriber rejection of emitted event
      Description: observability infrastructure rejects the emitted record even though the underlying business error remains valid.
    - Scenario: delegated emission chain broken
      Description: one layer in the composed error chain fails to forward emission correctly.
- Operation: `PariError` umbrella conversion
  - Failure
    - Scenario: wrong top-level operation classification
      Description: a public error is wrapped under the wrong top-level operation outcome.
    - Scenario: incompatible delegated source type
      Description: umbrella conversion receives an error source that does not match the expected operation family.
    - Scenario: umbrella mapping missing for a public operation
      Description: a caller-visible operation has no stable mapping into the umbrella error surface.

Design expectation:

- caller-visible primitive failures should almost never originate in `error`
- if emission fails, that failure should usually be contained to observability plumbing rather than replacing the original business error

### What this suggests for Primitive Error design

This operation catalog suggests the Primitive layer should probably be organized around a reusable set of primitive families rather than around today's enum boundaries.

The repeated primitive families already visible in the operation catalog are:

- identity / reference failures
- tracked-state failures
- request transport / actor boundary failures
- state-precondition failures
- schema / field-selection failures
- serialization / deserialization failures
- resolver / path-template failures
- codec encode / decode failures
- executor I/O failures
- validation rule violations
- cross-entity reference failures
- aggregation / batch-shape failures
- observability emission failures

Those families are a better starting point for designing Primitive Errors than the current source's public operation error enums.
