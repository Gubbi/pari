# Design Doc Migration — Status

Progress tracker for migrating from `docs/old_design/` to the new C4-aligned
doc set under `docs/design/`.

## C4 Mapping (current)

| C4 Level | What It Covers | Where It Lives |
|---|---|---|
| L1 System Context | Systems Pari is embedded in | Out of scope |
| L2 Container | Pari as a framework — seams, core, persistence | `docs/design/framework.md` |
| L3 Component | Formal layers and their internals | `docs/design/layers/` |
| L4 Code | Key types, interfaces, generation contracts | Rustdoc, co-located with code |

**Rule:** L3 is visually rich (Mermaid, tables) and references source by
`file:line`. L4 explains *what / why / usage*; mechanical descriptions stay
out — the code already says those. `CLAUDE.md` files are **derived context**
for agents, not sources of truth.

---

## Status

### Done

- [x] `docs/design/README.md` — index
- [x] `docs/design/framework.md` — L2 container view
- [x] `docs/design/layers/layer-model.md` — L3 generic framework (ownership, dependency rules, pure/orchestration split)
- [x] `docs/design/layers/error-handling.md` — L3 error layer (three-tier chain, classification, composition/emission split, primitive contract)
- [x] L4 rustdoc for error layer — `src/error/` + `pari-macros/src/{error_compose,otel_emit,activity_error_enum,primitive_error_enum,primitive_error}.rs`
- [x] Dropped intermediary-op tier from design and from `ErrorLayer` enum
- [x] `docs/design/CLAUDE.md` — authoring guidance + "CLAUDE.md is derived, not source of truth"
- [x] `src/error/CLAUDE.md` — refreshed to match new tier model

### Pending — per-layer design docs (L3)

Each needs an L3 doc under `docs/design/layers/` plus an L4 rustdoc pass
covering the infra types, key contracts, and relevant macros in `pari-macros/`.

- [ ] `layers/entities.md` — identity, tracked fields, schemas, `#[derive(Entity)]` codegen
- [ ] `layers/workspace.md` — uniform access gateway, generated accessors/setters, transparent expansion, automatic validation
- [ ] `layers/store.md` — `EntityServer` + `StoreManager` split, actor model, checkout lifecycle, sparse staging
- [ ] `layers/substrate.md` — asset pipeline (slot / asset / entity composition), schema-driven load/persist paths, backend implementations
- [ ] `layers/validation.md` — three-kind model (structural / semantic / cross-entity), `ValidationSchema<T>`, runner flow, `EntityClient`-calling cross-entity rule pattern

### Pending — stale CLAUDE.md refreshes

These drifted during implementation — plan to refresh as each layer's design
doc is authored (so the CLAUDE.md reflects the same reality).

- [ ] `src/store/CLAUDE.md` — module map references removed files (`server.rs`, `state.rs`, `op_error.rs`); should reference `entity_server.rs`, `manager.rs`
- [ ] `src/workspace/CLAUDE.md` — remove reference to `workspace/error.rs` (file was removed; errors re-exported from store layer)
- [ ] `src/validation/CLAUDE.md` — fix `ValidationKind` source to `src/validation/kind.rs`
- [ ] `src/substrate/CLAUDE.md` — update design doc links
- [ ] `CLAUDE.md` (root) — update "Useful References" to new `docs/design/` paths

### Pending — cleanup

- [ ] Remove `docs/old_design/` once every new doc is in place and we are confident nothing else needs to be mined from it.

---

## Working Order

One layer at a time, each as its own atomic unit:

1. Pick a layer from the pending list.
2. Mine `docs/old_design/<layer>_layer/` for concepts worth preserving.
3. Draft the L3 doc; discuss topic-by-topic before writing.
4. Walk the layer's source + related `pari-macros/` generation; add L4 rustdoc.
5. Refresh the layer's `CLAUDE.md` to match.
6. Commit.
