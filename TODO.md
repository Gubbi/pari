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

## Phase 1 — Functional Tests

One file per user job under `tests/functional/`. Every
persistence-meaningful scenario runs persist + reload. Where the
substrate is incidental, the scenario runs against both backends via
the parameterization fixture.

Order:

1. `author_role.rs`
2. `author_team.rs`
3. `author_workflow.rs`
4. `modify_persisted_entity.rs`
5. `author_workflow_with_intercepts.rs`
6. `author_embedded_workflow.rs`
7. `author_reusable_workflow.rs` + `author_relay.rs`
8. `validation_failures.rs`

End of phase: CLAUDE.md sweep across `src/`, `tests/`.

## Phase 2 — Integration Tests (Deferred)

Only added when a real boundary-failure mode resists end-to-end
coverage. Examples (illustrative): channel-closed mid-operation on the
workspace ↔ store seam; partial-substrate-response merge paths.

## Phase 3 — Unit Tests (Deferred)

Logic-heavy pure functions, future-proof assumptions, combinatorial
coverage. Colocated as `#[cfg(test)] mod tests` in the source file.

## Cleanup

- Remove `docs/old_design/`.
