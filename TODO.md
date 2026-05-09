# TODO — Test Implementation Plan

Phased plan for rebuilding test coverage from scratch, following the
principles in [docs/design/test.md](docs/design/test.md).

Each phase ends with a CLAUDE.md sweep across affected layers.

## Phase 0 — Foundations

Land the prerequisites before writing any new test.

### 0.1 Adopt rstest ✅

- Add `rstest` to `[dev-dependencies]` in `Cargo.toml`.
- Used for substrate parameterization and parameterized scenarios.

### 0.2 Runtime-agnostic refactor ✅

The library must not pull `tokio` into production. Drop the
`EntityServer` actor loop entirely; `EntityServer` becomes a stateless
dispatcher over the singleton `StoreManager`.

Change surface:

- `src/store/entity_server.rs`
  - Drop the per-server actor loop, `FuturesUnordered`, and `tokio::select!`.
  - `EntityServer` becomes stateless: holds `Arc<S>` and a handle to the
    process-wide `StoreManager` sender. `&self` methods dispatch directly.
  - Multiple `EntityServer` instances may exist; the `StoreManager` is
    the singleton.
- `src/store/lib/message.rs`
  - Delete `StoreMessage` (workspace ↔ server channel disappears).
  - `StoreManagerMessage` keeps using a oneshot reply, switched to
    `futures::channel::oneshot`.
- `src/workspace/lib/request.rs`
  - Replace the per-request oneshot pair with a direct dispatch call
    into the active `EntityServer`.
- `init` API
  - `init(substrate, spawn_fn)` accepts a caller-provided spawner.
  - The `StoreManager` future is spawned via `spawn_fn`; nothing is
    returned to the caller for it to drive.
  - Public surface: `pub type SpawnFn = Arc<dyn Fn(BoxFuture<'static, ()>) + Send + Sync>;`
- `EntityServer::with(...)` (test helper)
  - Drives the `StoreManager` future internally via `futures::join!` so
    tests do not need a runtime-specific spawner.
- Channel swap
  - Replace `tokio::sync::mpsc` / `tokio::sync::oneshot` with
    `futures::channel::mpsc` / `futures::channel::oneshot` throughout.
- Cargo
  - Move `tokio` from `[dependencies]` to `[dev-dependencies]`.
  - Production code depends on `futures` only.
- Design docs to update for the dispatcher shape
  - [docs/design/framework.md](docs/design/framework.md) — store-layer one-liner.
  - [docs/design/layers/layer-model.md](docs/design/layers/layer-model.md) — store-layer one-liner.
  - [docs/design/layers/store.md](docs/design/layers/store.md) — diagrams + prose:
    drop the workspace ↔ server channel; `EntityServer` is a stateless
    dispatcher; `StoreManager` is the singleton actor.
  - [docs/design/layers/workspace.md](docs/design/layers/workspace.md) — diagram update.

### 0.3 Test scaffolding ✅

- Single integration binary at `tests/tests.rs` declaring `common`,
  `fixtures`, and `functional` submodules via `#[path]`. Link cost paid
  once.
- `tests/fixtures/<entity>.rs` — one file per entity kind. Builders and
  canonical sample data only. No assertion helpers, no setup
  orchestration. Files added as user jobs require them.
- Substrate parameterization helper at `tests/common/substrate.rs` —
  `SubstrateKind` enum and `run_with(kind, scenario)` driving
  `RepoSubstrate` (over a tempdir) and `InMemorySubstrate`. Tests use
  `rstest` `#[case]` to fan a scenario across both. At least one
  scenario per user job runs against both.

### 0.4 CLAUDE.md sweep ✅

Refresh `src/store/CLAUDE.md`, `src/workspace/CLAUDE.md`, and the root
`CLAUDE.md` for the dispatcher shape and the runtime-agnostic stance.

## Phase 1 — Functional Tests ✅

One file per user job under `tests/functional/`. Every
persistence-meaningful scenario runs persist + reload. Where the
substrate is incidental, the scenario runs against both backends via
the parameterization fixture.

Originally-planned user jobs:

1. `author_role.rs` ✅
2. `author_team.rs` ✅
3. `author_workflow.rs` ✅
4. `modify_persisted_entity.rs` ✅
5. `author_workflow_with_intercepts.rs` ✅
6. `author_embedded_workflow.rs` ✅
7. `author_reusable_workflow.rs` + `author_relay.rs` ✅
8. `validation_failures.rs` ✅

Additional user-job and concern files that landed during Phase 1:

- `lifecycle_failures.rs` — store-layer lifecycle preconditions.
- `validation_timing.rs` — validation tier × lifecycle moment.
- `abandon_in_progress_edit.rs` — `undo_checkout` happy path.
- `rollback_staged_change.rs` — `undo_commit` happy paths
  (added-entity removal, modified-entity revert).
- `refresh_entity_from_substrate.rs` — `unload` happy paths,
  including external-edit refetch on `RepoSubstrate`.

Source-code changes that landed alongside the tests:

- Mutation isolation via per-entity typed `XDelegate` (typestate
  enforcement of "only checked-out entities are mutable").
- Insert and commit lifecycle preconditions enforced
  (`EntityAlreadyExists`, `EntityNotCheckedOut`).
- Embedded entities cross-entity-validate `entity_ref.parent`.
- Workflows can have empty steps (relaxation of the non-empty rule)
  to support iterative authoring.
- `Team.include` reshaped from `HashMap<EntityRef, EntityRef>` to
  `Vec<(EntityRef, EntityRef)>` with a duplicate-team rule
  (the JSON intermediate cannot represent struct-keyed maps).
- `camel_case` validation rule renamed to `pascal_case` to match
  what the regex actually validates.
- `step_keys_pascal_case` structural rule added for workflow steps.
- Common entity types relocated under `common/` in the repo
  substrate (`common/roles/`, `common/hooks/`, `common/teams/`,
  `common/artifact-kinds/`, `common/workflows/` for reusable).
- End-of-phase CLAUDE.md sweep across `src/`, `pari-macros/`, and
  `tests/`. Authoring constraints (avoid struct-keyed maps;
  iterative authoring of cross-referenced trees) captured in
  `docs/design/layers/entities.md`.

## Phase 1.5 — Schema & Extensions Coverage

Tests for the schema-validation gate and `Extensions` `x-` prefix
behavior landed in commits `d0f41fe`, `3063575`, `1ce2254`, `ea979c6`.
None of this work has functional coverage today.

### 1.5.1 `import_from_json.rs` — functional (e2e)

User job: import a raw-JSON entity into a workspace via
`Workspace::import_json`. One file, mirroring
`validation_failures.rs` style (sectioned by concern). Runs against
both substrate backends where persistence-meaningful.

Sections and test cases:

- **Happy path** (one per representative entity kind; runs against
  both substrate backends):
  - `import_json_role_round_trips_through_persist`
  - `import_json_team_round_trips_through_persist`
  - `import_json_workflow_round_trips_through_persist`
- **Schema rejections** (each surfaces a schema error, not a
  downstream structural/semantic error):
  - `import_json_rejects_missing_required_field`
  - `import_json_rejects_wrong_json_type`
  - `import_json_rejects_unknown_top_level_field`
  - `import_json_rejects_bare_extension_key`
- **Validation ordering** (schema gate runs first; later tiers
  still fire):
  - `import_json_schema_valid_then_structural_failure_surfaces_structural_error`
  - `import_json_schema_and_structural_valid_then_semantic_failure_surfaces_semantic_error`
- **Extensions round-trip.** Prefix logic is mechanical:
  serialize prepends `x-`, deserialize strips the first `x-`
  (so wire `x-x-foo` ↔ in-memory `x-foo`):
  - `import_json_strips_x_prefix_on_extension_keys`
  - `serialize_prepends_x_prefix_on_extension_keys`
  - `extensions_round_trip_preserves_multiple_and_nested_values`
  - `extensions_round_trip_handles_empty_map`
  - `extensions_double_x_prefix_round_trip`

## Phase 1.6 — Codec / Slot Refactor ✅

Three-step landing:

1. **Common ✅** (`f99d7b5`). `Codec::decode` returns `serde_json::Value`
   (wire-shaped object). `merge_field_map_into_json` adapted to take a
   `Value`. Thin shims in both codec impls; behavior unchanged.
2. **Repo ✅** (`34453c3`). Added `FlattenRule::Prefix`,
   `RepoSlot::FrontmatterFlattened(rule)`,
   `RepoSlot::SectionFlattened(rule, content)`. RepoCodec rewritten
   for longest-prefix-match routing on unclaimed wire keys; unmatched
   keys error at codec-level. `TASK_FIELDS` carries both flatten
   variants (`Prefix("x-")` for frontmatter, `Prefix("x-doc-")` for
   sections) demonstrating same-key co-ownership of one struct field.
   Pipeline schema invariant relaxed for flattened slots sharing a
   key. Workspace validation runner stops rejecting fields with no
   rules (load path may pass extension-bag fields).
   `substrate_load_boundary` tests flipped from 3 ignored → all green.
3. **In-memory ✅** (this commit). `FlattenRule` hoisted into
   `pipeline/slot.rs`. Added `ValueSlot::Flattened(FlattenRule)`.
   InMemoryCodec rewritten to mirror repo's prefix-match routing.
   Schema entries for `extensions` updated to
   `ValueSlot::Flattened(Prefix("x-"))` across all entities. Added
   `role_with_extensions_round_trips_through_persist` parametrized
   over both backends — covers the original missing e2e: extensions
   inserted with bare keys round-trip through persist + reload.

## Phase 2 — Integration Tests ✅

Boundary-failure modes that resist functional coverage. Only the
genuinely-orthogonal cases were kept; routine variations covered by
the codec being generic across entity kinds were dropped per the
"coverage over exhaustiveness" principle.

- **Substrate corruption.** Existing tests in
  [tests/functional/external_corruption.rs](tests/functional/external_corruption.rs)
  cover malformed frontmatter, unterminated frontmatter, garbage
  content, and (added during this phase) externally-deleted files
  (`a68ac5a`). Cases for missing-required, wrong-type, and unknown
  fields are pinned in
  [tests/functional/substrate_load_boundary.rs](tests/functional/substrate_load_boundary.rs)
  alongside the schema gate work from Phase 1.6. Other entity kinds,
  H1-specific corruption, and multi-asset Task corruption skipped:
  same code path as Role, no new coverage.
- **Channel-closed → `StoreUnavailable`** ✅ (`8da525e`).
  [tests/functional/store_unavailable.rs](tests/functional/store_unavailable.rs)
  covers the workspace ↔ store seam with two harnesses: a
  `BrokenStoreDispatcher` that fails immediately (resolve / persist
  paths) and a `ToggleStoreDispatcher` for mid-session actor drop
  (field-accessor path).
- **Partial substrate-response merge paths** ✅ (`d3e7dd9`).
  Contract pinned in
  [tests/functional/sparse_substrate_response.rs](tests/functional/sparse_substrate_response.rs):
  required-missing surfaces a schema-gate rejection;
  optional-missing fills `null` so the field is loaded and accessors
  don't re-issue Load. `defaults::load` gained a fill-in-null pass
  after slice validation.
- **Substrate-side schema gate at load** ✅ (Phase 1.6, `34453c3`).
  Cases in [substrate_load_boundary.rs](tests/functional/substrate_load_boundary.rs).
- **Extensions `x-` prefix at the disk boundary** ✅ (Phase 1.6 +
  `34453c3`, `5e2c048`). Repo-side disk-shape pin in
  `repo_substrate_writes_x_doc_extension_to_section`; in-memory
  round-trip in `role_with_extensions_round_trips_through_persist`.
- **Generated-schema artifacts** ✅ (CI gate, not a dev test).
  Drift + structural invariants both wired into the `schemas` job in
  [.github/workflows/ci.yml](.github/workflows/ci.yml) via
  `cargo xtask generate-schemas` + `cargo xtask check-schemas`.

## Phase 3 — Unit Tests (Deferred)

Logic-heavy pure functions, future-proof assumptions, combinatorial
coverage. Colocated as `#[cfg(test)] mod tests` in the source file.

## Cleanup

- Remove `docs/old_design/`.
