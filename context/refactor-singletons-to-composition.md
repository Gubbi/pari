# Refactor: remove singletons, compose components

Working notes capturing per-layer decisions for the refactor that
removes ambient state (`GLOBAL_ENTITY_SERVER`, thread-local override)
and reshapes the workspace/store boundary around explicit composition.

This document is the holistic statement after all open questions were
resolved. It will be folded into proper proposals/specs later.

## Guiding principles

1. **Tracked entities are state.** A `TrackedX<T>` is pure state —
   loaded fields, dirty tracking, JSON round-trip. It outlives any
   workspace and is portable across them. It has no dispatcher, no
   workspace reference, no async accessors.
2. **A `Workspace` is a bounded session of entity work.** It binds a
   set of in-scope entities to a server (via a dispatcher) for some
   period of work — read-only or read-write. Anyone can construct one
   over a dispatcher. Multiple workspaces coexist. Validators do not
   have their own workspace; they operate inside the workspace that
   invoked them.
3. **`XViewer<'ws, T>` and `XEditor<'ws, T>` are workspace-bound
   handles** to a tracked entity. They cannot escape the workspace's
   lifetime. `XViewer` exposes typed async accessors (read);
   `XEditor` wraps an `XViewer` and adds typed async setters and the
   `commit` / `undo_checkout` lifecycle (read-write, returned only by
   checkout).
4. **The workspace dispatch surface is stateless w.r.t. any handle.**
   `Workspace`'s public methods take typed refs or plain entities,
   never a `TrackedX`/`XViewer`/`XEditor`. Methods that operate on a
   specific handle live on that handle.
5. **Store layer always deals with type-erased entities.**
   `WorkspaceRequest`, `WorkspaceResponse`, `StoreRequest`,
   `StoreResponse`, and every server- and store-side surface speak
   only `TrackedEntity` and `AnyEntityRef`. No method on `StoreServer`
   or `Store` is generic over `T: Entity`.
6. **Workspace performs all type ↔ type-erased conversions** at its
   public surface (input refs, output entities) and at the
   workspace↔store boundary. Typed ergonomics are a workspace concern
   only.

## Cross-cutting decisions

- **Caller-facing handle is `Workspace`** (no separate `EntityClient`
  type). `Workspace::new(dispatcher)` is the entry point.
- **Two trait boundaries with parallel structure**, both in the store
  layer:
  - `Dispatcher` — workspace → server (`dispatch(WorkspaceRequest)`).
  - `StoreDispatcher` — server → store actor
    (`dispatch(StoreRequest)`).
- **Each component returns its handle from its constructor**:
  `Store::start(spawn_fn) -> Arc<dyn StoreDispatcher>`,
  `StoreServer::start(substrate, store) -> Arc<dyn Dispatcher>`.
- **Vocabulary uniformity**: `revert` / `forget` at every layer — in
  request variants, internal methods, and public `Workspace` API.
- **JSON across the wire**: `WorkspaceRequest::Insert` carries
  `serde_json::Value`. Substrate already deals in JSON; this unifies
  the deserialization seam. A single `StoreServer` helper
  (`json_to_tracked`) is used by both the insert path and the
  substrate-load path — see Item 7.
- **Substrate boundary is fully ref-typed**: `Substrate::load_strategy`
  and `Substrate::schema_for` take `&AnyEntityRef`, not `EntityKind`.
  `EntityKind` becomes substrate-internal vocabulary.
- **`AnyEntityRef` conversion via instance method**:
  `entity_ref.to_any_ref()` (replaces `<X as Entity>::to_any_ref(...)`).
- **`SpawnFn`** continues; library remains runtime-agnostic.
- All singleton machinery removed: `GLOBAL_ENTITY_SERVER`,
  `OVERRIDE_ENTITY_SERVER`, `active_entity_server`,
  `install_global_entity_server`, `install_override_entity_server`,
  `OverrideGuard`, `pari::init`, `pari::with`,
  `workspace::lib::request::request`.

## Construction shape

```rust
let store: Arc<dyn StoreDispatcher>     = Store::start(spawn_fn);
let server: Arc<dyn Dispatcher>         = StoreServer::start(substrate, store);
let workspace: Workspace                = Workspace::new(server.clone());

// callers use the workspace directly:
let viewer = workspace.resolve(role_ref).await?;
let editor = workspace.checkout(role_ref).await?;
```

No globals. Each component independently constructed; alternate
implementations slot in at any boundary.

## Layer model

- `entity` — state types, refs, kinds.
- `store` — dispatcher traits, request/response protocol, `Store`,
  `StoreServer`. Depends on entity, substrate.
- `substrate` — persistence trait + backends. Depends on entity.
- `workspace` — `Workspace`, `XViewer`, `XEditor`, `Validator`,
  validation rules. Depends on entity, store.
- `error`, `test` — as today.

`Workspace` is used both client-side (caller code) and server-side
(`StoreServer.handle` constructs per-request workspaces to invoke
validation). Same type, two roles.

**Mutual `workspace` ↔ `store` dependency** is the only documented
layer-model exception, justified by: workspace dispatches operations
*down* through `store::Dispatcher`; store invokes validation *up*
through workspace's `Workspace::import(...).validate()` flow. No other
cycles. No `workspace` ↔ `substrate` dependency.

---

## Item 1 — `Store` (was `StoreManager`)

### Renames

| Was | Now |
|---|---|
| `StoreManager` | `Store` |
| `StoreManagerRequest` | `StoreRequest` |
| `StoreManagerResponse` | `StoreResponse` |
| `StoreManagerMessage` | `StoreMessage` (internal envelope) |
| `StoreManagerRequest::UndoCommit` | `StoreRequest::Revert` |
| `StoreManagerRequest::UnloadEntity` | `StoreRequest::Forget` |
| `StoreManager::undo_commit` | `Store::revert` |
| `StoreManager::unload_entity` | `Store::forget` |

The pre-existing public types named `StoreRequest` / `StoreResponse`
are renamed in Item 2 to free these names.

### New trait (internal)

```rust
// store layer, internal — not exposed to workspace
pub trait StoreDispatcher: Send + Sync {
    fn dispatch(&self, req: StoreRequest) -> BoxFuture<'_, StoreResponse>;
}
```

### Lifecycle

```rust
pub fn start(spawn_fn: SpawnFn) -> Arc<dyn StoreDispatcher>;
```

`Store::start`:
- creates the mpsc channel,
- constructs the `Store` value,
- spawns the run loop via `spawn_fn`,
- wraps the sender in a `StoreDispatcher` impl that creates a oneshot
  per call,
- returns `Arc<dyn StoreDispatcher>`.

`StoreMessage { request, reply: oneshot }` is the wire envelope inside
the channel. Not visible to `StoreDispatcher::dispatch` callers.

### Behavior unchanged

- Type-erased throughout: no `EntityKind`, no `TypeId`, no generics.
  All requests/responses carry `TrackedEntity` and `AnyEntityRef`
  only.
- Substrate-free: substrate access stays in `StoreServer`.
- Internal handlers (`revert`, `forget`, `remove_entity`, etc.)
  unchanged in logic; renames only.

---

## Item 2 — `StoreServer` (was `EntityServer`)

### Renames

| Was | Now |
|---|---|
| `EntityServer` | `StoreServer` |
| `StoreRequest` (public) | `WorkspaceRequest` |
| `StoreResponse` (public) | `WorkspaceResponse` |
| `StoreRequest::UndoCommit` | `WorkspaceRequest::Revert` |
| `StoreRequest::Unload` | `WorkspaceRequest::Forget` |
| `EntityServer::undo_commit` | `StoreServer::revert` |
| `EntityServer::unload` | `StoreServer::forget` |

### Layer placement

`Dispatcher` and `StoreDispatcher` both live in the **store layer** —
the store layer's outward and inward interfaces respectively. The
workspace layer consumes `Dispatcher` but does not own it.

### Trait + handle split

```rust
// store layer
pub trait Dispatcher: Send + Sync {
    fn dispatch(&self, req: WorkspaceRequest) -> BoxFuture<'_, WorkspaceResponse>;
}

pub struct StoreServer<S> {
    substrate: Arc<S>,
    store: Arc<dyn StoreDispatcher>,
    self_dispatcher: Weak<dyn Dispatcher>,    // for per-request workspace construction
}

pub struct StoreServerHandle<S> {
    inner: Arc<StoreServer<S>>,
}

impl<S> Dispatcher for StoreServerHandle<S> { ... }
```

The split mirrors `Store`/`StoreHandle` and preserves future
flexibility for an actor on the StoreServer side.

### Lifecycle

```rust
pub fn start(substrate: S, store: Arc<dyn StoreDispatcher>) -> Arc<dyn Dispatcher> {
    Arc::new_cyclic(|weak: &Weak<StoreServerHandle<S>>| {
        let server = StoreServer {
            substrate: Arc::new(substrate),
            store,
            self_dispatcher: weak.clone() as Weak<dyn Dispatcher>,
        };
        StoreServerHandle { inner: Arc::new(server) }
    }) as Arc<dyn Dispatcher>
}
```

`StoreServerHandle` is what callers hold. `StoreServer` keeps a `Weak`
back-reference to its handle so it can construct a per-request
workspace over its own dispatcher. Strong cycle is avoided
(handle → server is strong; server → handle is weak).

### Behavior

- Stateless orchestration over substrate + store. No spawned task.
- Type-erased throughout: every public method on `StoreServer` and
  every variant of `WorkspaceRequest` / `WorkspaceResponse` speaks
  `TrackedEntity` and `AnyEntityRef` only.
- `dispatch(WorkspaceRequest)` matches and routes to per-op methods on
  the inner `StoreServer<S>` (`resolve`, `has_ref`, `insert`, `remove`,
  `checkout`, `load`, `ensure_mutable`, `persist`, `revert`, `forget`,
  `commit`).
- `store_send(...)` collapses to `self.store.dispatch(req).await`.
  `StoreMessage` envelope is no longer visible.

### Per-request workspace for validation

For operations that need validation (`insert`, `commit`, `load`),
`StoreServer.handle` constructs an ad-hoc `Workspace` over its own
dispatcher and routes validation through it:

```rust
let workspace = Workspace::new(self.self_dispatcher.upgrade().expect("..."));
let viewer = workspace.import(entity);     // type-erased import → typed viewer per kind
viewer.validate(...).await?;
// dispatch StoreRequest::* to Store on success
```

`Workspace::new` is cheap (one Arc clone + a static reference for the
validator), so per-call construction is fine.

(Persist does not validate — it trusts that prior commits were already
gated.)

### Unified JSON → verified tracked pipeline (DRY)

Two paths produce a verified `TrackedEntity` from JSON inside
`StoreServer`: the `insert` path (`WorkspaceRequest::Insert { json }`)
and the substrate-load path (`substrate.load(...) -> json`). Both share
the full pipeline JSON → tracked → import → validate via a single
helper, with a smaller pure-conversion helper underneath:

```rust
impl<S> StoreServer<S> {
    /// Pure conversion: JSON → TrackedEntity. No validation, no workspace.
    fn json_to_tracked_state(&self, json: serde_json::Value)
        -> Result<TrackedEntity, ActivityError>
    {
        // 1. Extract entity_ref from JSON; derive kind.
        // 2. Per-kind dispatch (via entity_registry!) deserializes JSON → typed plain.
        // 3. Construct typed TrackedX<X> with each field as TrackedField::loaded(value).
        // 4. Wrap into the matching TrackedEntity variant.
    }

    /// Full pipeline: JSON → TrackedEntity → workspace.import → viewer.validate_with.
    /// Returns the verified TrackedEntity for downstream dispatch into `Store`.
    async fn json_to_verified_tracked(
        &self,
        json: serde_json::Value,
        fields: &[&str],                  // [] = whole entity
        kinds: &[ValidationKind],
    ) -> Result<TrackedEntity, ActivityError> {
        let tracked = self.json_to_tracked_state(json)?;
        let workspace = Workspace::new(
            self.self_dispatcher.upgrade().expect("...")
        );
        let viewer = workspace.import_erased(tracked.clone());
        viewer.validate_with(fields, kinds).await?;
        Ok(tracked)
    }
}
```

Used by:
- `handle_insert` —
  `json_to_verified_tracked(json, &[], &[Structural, Semantic, CrossEntity])`,
  result handed to `Store::insert`.
- `load_fields` —
  `json_to_verified_tracked(loaded_json, &loaded_fields, &[Structural, Semantic])`,
  result handed to `Store::initialize_field`.

The `tracked.clone()` before `import_erased` is cheap — the outer
struct clone bumps per-field `Arc<TrackedField>` refcounts; no field
data is copied.

Substrate stops calling `TrackedEntity::from_json_value` directly; it
returns JSON and `StoreServer` does the wrap.

### `insert` shape — JSON across the wire

```rust
pub enum WorkspaceRequest {
    Insert { json: serde_json::Value },
    ...
}
```

The typed surface at `Workspace`:
```rust
async fn insert<T: Entity + Serialize>(&self, plain: T) -> Result<()> {
    let json = serde_json::to_value(plain)?;
    self.dispatcher.dispatch(WorkspaceRequest::Insert { json }).await ...
}
```

`StoreServer.handle_insert`: `json_to_tracked(json)` → per-request
workspace + import + validate → dispatch to `Store`.

### Substrate calls — fully ref-typed

- `S::load_strategy(any_ref.kind(), field)` → `S::load_strategy(any_ref, field)`
- `Sub::schema_for(any_ref.kind())` → `Sub::schema_for(any_ref)`

After this, `StoreServer` references `EntityKind` zero times.

### Removed (singleton machinery)

- `GLOBAL_ENTITY_SERVER`, `OVERRIDE_ENTITY_SERVER`, `active_entity_server`
- `install_global_entity_server`, `install_override_entity_server`
- `OverrideGuard`
- `workspace::lib::request::request`

---

## Item 3 — `TrackedX<T>` (entity layer)

Structurally unchanged — already a pure-state type-erased holder. The
only changes are visibility tightening and removal of the public
`From<PlainX>` constructor.

### Changes

- `From<PlainX> for TrackedX` — **removed** from public API. The only
  construction path into `TrackedX` is via `StoreServer::json_to_tracked`
  (which uses `TrackedField::loaded` per field internally).
- `pub(crate) fn make_stub(ref_)` — visibility narrowed; same body.
- All other state methods (`is_field_loaded`, `has_dirty_fields`,
  `dirty_fields`, `reset_dirty`, `merge_dirty_into`, `to_json_value`,
  field reads, `Clone`) — visibility tightened to `pub(crate)` to
  entity / store / substrate.
- Per-field async accessors (Bucket B) — **moved off** `TrackedX`.
  They now live on `XViewer<'ws, T>` (Item 5).

### `entity_ref().to_any_ref()` instance method

`EntityRef<X, P>` gains a `to_any_ref(&self) -> AnyEntityRef`
instance method (entity layer). Replaces the trait associated-fn form
`<X as Entity>::to_any_ref(...)` at all callsites in generated code
and in entity-layer helpers. Cleaner ergonomics; consistent with
`entity_ref()` returning a typed value the user can call methods on.

---

## Item 4 — `TrackedEntity` (entity layer)

### Changes

- `from_json_value(any_ref, value)` — visibility narrowed to
  `pub(crate)`; called only by `StoreServer::json_to_tracked`.
  Substrate stops constructing `TrackedEntity` directly.

No structural changes otherwise. Variants still wrap typed
`TrackedX<T>` values; pass-through `pub(crate)` methods unchanged.

---

## Item 5 — `XViewer<'ws, T>` and `XEditor<'ws, T>` (workspace layer)

### Three-tier vocabulary

| Type | Lifetime | Workspace binding | Capabilities |
|---|---|---|---|
| `TrackedX<T>` | Outlives workspace; portable | None | State only — fields, dirty tracking, json round-trip, `pub(crate)` to entity/store/substrate |
| `XViewer<'ws, T>` | Bounded by workspace | `&'ws Workspace` | Typed async read accessors, `validate()` |
| `XEditor<'ws, T>` | Bounded by workspace | `&'ws Workspace` (via wrapped viewer) | Typed async setters + `commit` / `undo_checkout` lifecycle, plus all `XViewer` capabilities via `Deref` |

### Shape

```rust
pub struct XViewer<'ws, T: Entity> {
    inner: TrackedX<T>,                // owned
    workspace: &'ws Workspace,
}

pub struct XEditor<'ws, T: Entity> {
    viewer: XViewer<'ws, T>,           // wraps the viewer
}

impl<'ws, T: Entity> Deref for XEditor<'ws, T> {
    type Target = XViewer<'ws, T>;
    fn deref(&self) -> &XViewer<'ws, T> { &self.viewer }
}
```

Read accessors are generated only on `XViewer`; `XEditor` inherits
them via `Deref`. Setters and lifecycle live only on `XEditor`. Ownership
of `TrackedX<T>` is shared with the store via per-field `Arc<TrackedField>`
— lazy-load propagates via the shared `OnceLock` interior, no outer
replacement needed.

### Generated accessor body (XViewer)

```rust
pub async fn role_id(&self) -> Result<&str, ActivityError> {
    if self.inner.role_id.get().is_none() {
        self.workspace.dispatcher.dispatch(
            WorkspaceRequest::Load {
                any_ref: self.inner.entity_ref().to_any_ref(),
                field: "role_id".into(),
            }
        ).await?...;
    }
    Ok(self.inner.role_id.get().expect("...").as_str())
}
```

### XViewer methods

```rust
impl<'ws, T: Entity> XViewer<'ws, T> {
    // generated per-field async accessors
    pub async fn <field>(&self) -> Result<&Type, ActivityError>;

    // workspace pass-through
    pub fn workspace(&self) -> &Workspace { self.workspace }
    pub fn entity_ref(&self) -> &EntityRef<T, T::Parent> { self.inner.entity_ref() }

    // validation — both route through self.workspace.validator on self.
    // `validate` runs the default kinds for this entity over all loaded fields.
    pub async fn validate(&self) -> Result<(), ActivityError>;

    // `validate_with` is parameterized for setter / load callers. `fields = []`
    // means "whole entity"; otherwise restricts field-scoped rules to those.
    pub async fn validate_with(&self, fields: &[&str], kinds: &[ValidationKind])
        -> Result<(), ActivityError>;
}
```

### XEditor methods

```rust
impl<'ws, T: Entity> XEditor<'ws, T> {
    // generated per-field async setters
    pub async fn set_<field>(&mut self, value: ...) -> Result<()>;

    pub async fn commit(self) -> Result<()>;          // consumes
    pub async fn undo_checkout(self) -> Result<()>;   // consumes

    // read accessors and validate() inherited via Deref<Target = XViewer>
}
```

Not `Clone`. Consumes self on lifecycle terminators. Borrow-bound to
workspace through the wrapped viewer.

### Setter pattern (preserved from today, adapted)

1. Clone `self.viewer.inner` into a candidate `TrackedX<T>`.
2. Replace the candidate's `<field>` slot with
   `Arc::new(TrackedField::mutated(value))`.
3. Wrap the candidate in a transient `XViewer` over the same workspace
   and run `Structural` + `Semantic` validation against it.
4. On success, swap the new field-Arc into `self.viewer.inner.<field>`.
   On failure, drop the candidate; `self.viewer.inner` is unchanged.

Cross-entity validation runs at `commit` (server-side via the
per-request workspace).

### Macro responsibility

- `entity_codegen.rs` (entity layer): `TrackedX<T>` struct with state
  fields and `pub(crate)` methods.
- `workspace_codegen.rs` (workspace layer): `XViewer<'ws, T>` struct
  with all per-field typed async accessors, `XEditor<'ws, T>` struct
  with `Deref` impl + per-field typed setters + `commit` /
  `undo_checkout`.

---

## Item 6 — `Workspace` (workspace layer)

### Shape

```rust
pub struct Workspace {
    dispatcher: Arc<dyn Dispatcher>,
    validator: Validator,
}

impl Workspace {
    pub fn new(dispatcher: Arc<dyn Dispatcher>) -> Self {
        Workspace { dispatcher, validator: Validator::new() }
    }

    // Reads — typed in/out
    pub async fn resolve<T: Entity>(&self, ref_: EntityRef<T, T::Parent>)
        -> Result<XViewer<'_, T>, ActivityError>;

    pub async fn has_ref<T: Entity>(&self, ref_: EntityRef<T, T::Parent>)
        -> Result<bool, ActivityError>;

    // Writes — typed in/out
    pub async fn insert<T: Entity + Serialize>(&self, plain: T)
        -> Result<(), ActivityError>;
    pub async fn checkout<T: Entity>(&self, ref_: EntityRef<T, T::Parent>)
        -> Result<XEditor<'_, T>, ActivityError>;
    pub async fn remove<T: Entity>(&self, ref_: EntityRef<T, T::Parent>)
        -> Result<XViewer<'_, T>, ActivityError>;
    pub async fn persist(&self) -> Result<(), ActivityError>;
    pub async fn revert_and_forget<T: Entity>(&self, ref_: EntityRef<T, T::Parent>)
        -> Result<(), ActivityError>;
    pub async fn forget<T: Entity>(&self, ref_: EntityRef<T, T::Parent>)
        -> Result<(), ActivityError>;

    // Transient handling — typed in/out
    pub fn import<T: Entity>(&self, tracked: TrackedX<T>) -> XViewer<'_, T>;

    // Type-erased import for server-side use (called by StoreServer.handle).
    // Wraps a TrackedEntity into the matching typed XViewer per variant.
    pub(crate) fn import_erased(&self, tracked: TrackedEntity) -> ErasedXViewer<'_>;
}
```

### Properties

- Plain owned value (not always-Arc'd). `Workspace::new` is cheap.
- Strongly tied to one server (one `Arc<dyn Dispatcher>`).
- Anyone can construct one over a dispatcher.
- Validator is built once during construction (just stamps a static
  rule-registry reference); reused across all validations performed by
  this workspace.
- `XViewer`/`XEditor` borrow the workspace and cannot outlive it.

### Type ↔ type-erased conversion responsibility

Workspace is the **only place** that converts between typed and
type-erased forms:

- Public methods take typed `EntityRef<T, T::Parent>` and return typed
  `XViewer<'_, T>` / `XEditor<'_, T>` / `bool`.
- Inside method bodies, the typed ref is converted to `AnyEntityRef`
  via `ref_.to_any_ref()` before dispatching `WorkspaceRequest::*`.
- The dispatcher returns `WorkspaceResponse` carrying `TrackedEntity`
  (type-erased); the workspace extracts the typed `TrackedX<T>` via
  per-kind dispatch (using `T::extract` or equivalent) before wrapping
  into a typed `XViewer`/`XEditor`.

`StoreServer` and `Store` never see typed values.

### Notes on specific methods

- `remove` returns `XViewer<'_, T>` of the just-removed entity. The
  underlying entity is no longer in the store, so lazy-loading any
  unloaded fields will error. Callers should ensure the fields they
  care about were loaded prior to remove.
- `import` consumes a `TrackedX<T>` (typed plain state) and wraps it
  as a transient read-only viewer bound to this workspace. Use case:
  validation of transient entities not yet in the store, and
  server-side wrapping of a `TrackedEntity` received in
  `WorkspaceRequest::Commit` for cross-entity validation.
- **No direct `Workspace::validate(...)` API.** Validation is reachable
  only through `XViewer::validate()` and `XEditor::validate()`
  (latter via `Deref`). For transient entities,
  `workspace.import(tracked).validate().await?` is the path.

### Why no `EntityClient`

`EntityClient` collapsed entirely. Its methods all live on `Workspace`
now (plus on `XViewer`/`XEditor` for handle-specific operations). No
typed-sugar dispatcher wrapper exists separately from `Workspace`.

---

## Item 7 — `Validator` and validation rules (workspace layer)

### `Validator` shape

```rust
static REGISTRY: LazyLock<ValidationRuleSet> = LazyLock::new(|| {
    let mut rs = ValidationRuleSet::new();
    role::register(&mut rs);
    team::register(&mut rs);
    // ... per-entity registration
    rs
});

pub struct Validator {
    rules: &'static ValidationRuleSet,
}

impl Validator {
    pub fn new() -> Self { Self { rules: &REGISTRY } }

    pub(crate) async fn run<T: Entity>(
        &self,
        viewer: &XViewer<'_, T>,
        fields: &[&str],
        kinds: &[ValidationKind],
    ) -> Result<(), ActivityError>;
}
```

`Validator::new()` is effectively free — just stamps a `'static`
reference. `Workspace::new` is cheap by extension.

### Rule signatures

- **Structural** rules — sync, take field value, return errors.
  Unaffected.
- **Semantic** rules — receive `&XViewer<'_, T>`, read sibling fields
  via typed async accessors (lazy-load uniform).
- **Cross-entity** rules — receive `&XViewer<'_, T>`, read own fields
  via typed accessors, resolve siblings via
  `viewer.workspace().resolve(other_ref).await` (returns another
  `XViewer` bound to the same workspace).

### Validation on an `XEditor`

`XEditor` exposes `validate()` via `Deref` to `XViewer`. Where rule
signatures take `&XViewer<'_, T>`, callers can pass `&*editor` (or rely
on Deref coercion). For setter pre-validation, the setter constructs a
transient `XViewer` over the cloned candidate and passes it to the
validator (Item 5 setter pattern).

### Validation invocation sites

| Site | Where | What runs |
|---|---|---|
| Setter (per-field) | workspace, in `XEditor::set_<field>` | Structural + Semantic on transient candidate viewer |
| Insert | server, in `StoreServer.handle_insert` via per-request workspace | Structural + Semantic + Cross-entity |
| Commit | server, in `StoreServer.handle_commit` via per-request workspace | Cross-entity |
| Load | server, in `StoreServer.load_fields` via per-request workspace | Structural + Semantic on loaded fields |
| Persist | (none) | Trusts prior validation gates |

### Layer placement

Validation rules and the `Validator` type **live in the workspace
layer**. Rule files (`cross_entity/team.rs`, `cross_entity/relay.rs`,
etc.) move into the workspace layer (or stay at `src/validation/`
re-designated as workspace-layer in CLAUDE.md and the layer model).

### Removal of singleton callsites

`validation/lib/rules/cross_entity/*.rs` no longer imports
`EntityClient`. All paths flow through the viewer's workspace.

---

## Item 8 — Substrate

### `Substrate` trait — fully ref-typed

```rust
pub trait Substrate: ... {
    fn load_strategy(&self, any_ref: &AnyEntityRef, field: &str)
        -> Result<LoadStrategy, ActivityError>;
    fn schema_for(&self, any_ref: &AnyEntityRef)
        -> &'static EntitySchema<Self::Slot>;
    // ... existing methods unchanged, EntityKind references replaced with &AnyEntityRef
}
```

### Deserialization seam

Substrate produces `serde_json::Value` (already does today). It does
**not** construct `TrackedEntity` directly anymore. `StoreServer`
receives the JSON and constructs the bound `TrackedEntity` via its
`json_to_tracked` helper (see Item 2 — DRY consolidation).

### Internal `EntityKind` use

`schema_registry.rs` keeps the per-kind dispatch (`Role | Hook | …`)
as substrate's internal vocabulary. `EntityKind` is no longer named
outside substrate.

---

## Future plans (documented, not in scope now)

The workspace-as-bounded-session model naturally extends to
all-or-nothing transactional semantics over multiple entities. Out of
scope for this refactor, but the structure here makes them
implementable later without redesign:

- `Workspace::commit_all()` — commit every in-flight `XEditor`
  atomically; validation runs across all in-scope entities first.
- `Workspace::discard()` — drop all in-flight checkouts; reset
  workspace to a clean state.
- `Workspace::persist()` — persist all dirty entities the workspace
  has touched; rollback on validation failure for any.

These operations require:
- Workspace-side tracking of in-flight editors/viewers.
- Cross-entity validation orchestration over the workspace's full
  in-scope set.
- A defined workspace lifecycle ("end cycle") covering commit/discard/
  persist/close.

Today the workspace is a thin handle (dispatcher + validator). It will
thicken when we build out these semantics, but doing so does not
require structural reshaping of the Workspace/XViewer/XEditor split
established here.

## Open questions

All resolved.

1. ~~Per-entity accessor declaration placement.~~ Validation moved
   into workspace layer; `XViewer<T>` is workspace-layer; rules take
   `&XViewer<'_, T>`; validator uses the workspace held by the viewer.
2. ~~`StoreServer` per-request workspace lifetime.~~ Ad-hoc per call;
   `Validator::new()` is cheap (static rule registry).
3. ~~`XViewer<'ws, T>` ownership of inner.~~ Each viewer/editor owns a
   `TrackedX<T>`; per-field `Arc<TrackedField>` shared with store;
   lazy-load propagates via shared `OnceLock` interior.
4. ~~`XEditor` exposes read accessors how?~~ `XEditor` wraps `XViewer`
   and `Deref`s to it; accessors generated only on `XViewer`.
5. ~~Macro split.~~ No trait/impl split needed; existing
   `entity_codegen.rs` / `workspace_codegen.rs` separation stands.
