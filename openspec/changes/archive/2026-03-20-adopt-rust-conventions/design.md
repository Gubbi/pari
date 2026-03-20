## Context

The codebase currently has:
- `ValidationError` and `SubstrateError` as plain structs with no `Display` or `std::error::Error` impl
- Entity `id` fields typed as `String`, with format validation only in the validation layer
- No `rustfmt.toml` — formatting is unenforced
- No clippy lint configuration — pedantic lints are silently ignored
- No module-level doc comments
- No dependency auditing (`cargo deny`)

These are independent, low-risk changes that can be applied atomically.

## Goals / Non-Goals

**Goals:**
- `ValidationError` and `SubstrateError` implement `Display` + `std::error::Error` via `thiserror`
- All entity ID fields use newtypes: `RoleId`, `TeamId`, `HookId`, `WorkflowId`, `TaskId`, `RelayId`
- Each newtype carries serde/schemars attributes transparently
- All source files formatted under a consistent `rustfmt.toml`
- `clippy::pedantic` warnings enabled crate-wide; all resulting lints resolved
- `//!` doc comments on every source module
- `deny.toml` in place for license and advisory enforcement

**Non-Goals:**
- Changing validation logic or error semantics — `ValidationError` shape (path + message) is unchanged
- Adding `cargo deny` to a CI pipeline — tooling adoption only; pipeline is out of scope
- `thiserror` on `ValidationError` as an `enum` — it stays a struct; `thiserror` supports structs too

## Decisions

### D1: Newtype IDs live in `src/schema/ids.rs`

A dedicated module keeps all ID types grouped and importable without pulling in all of `types.rs`. Each newtype is `#[serde(transparent)]` so it serializes as a plain string. The `#[schemars(regex(...))]` annotation moves from the entity struct field to the inner field of the newtype, so the generated schema retains the format constraint.

```
RoleId(String)     — kebab-case  ^[a-z][a-z0-9-]*$
TeamId(String)     — kebab-case
HookId(String)     — kebab-case
WorkflowId(String) — CamelCase  ^[A-Z][A-Za-z0-9]*$
TaskId(String)     — CamelCase
RelayId(String)    — CamelCase
```

Task and Relay are embedded-only (no standalone schemas), but their `id` fields still benefit from type safety — cross-referencing within a workflow's steps by ID becomes compile-checked.

Alternative considered: inline newtypes in each entity file. Rejected — IDs are cross-referenced (e.g. Role ids appear in Team members, step ids in `depends_on`), so centralising avoids duplication.

### D2: `thiserror` on both error types, struct form for `ValidationError`

`thiserror` supports both enums and structs. `ValidationError` stays a struct (single variant, two fields). The `#[error("{message} at {path}")]` format string covers the `Display` impl without changing the data model.

`SubstrateError` is also a struct today; same approach applies.

### D3: `clippy::pedantic` via crate-level attribute, not `Cargo.toml`

`#![warn(clippy::pedantic)]` in `lib.rs` is checked in and version-controlled alongside the code it governs. Individual `#[allow(...)]` suppressions at the call site are preferred over blanket allows, except for high-noise lints (`clippy::module_name_repetitions`) that may be suppressed crate-wide.

### D4: `deny.toml` — advisories + licenses, no bans initially

Start with:
- `[advisories]` — vulnerability and unsound crate checking at `deny` level
- `[licenses]` — allow MIT, Apache-2.0, BSD-2-Clause, BSD-3-Clause, ISC, Unicode-3.0; deny unlicensed
- `[bans]` — no banned crates initially; `multiple-versions = "warn"` to surface duplication
- `[sources]` — allow crates.io only

## Risks / Trade-offs

- **Newtype ID is BREAKING for direct struct construction** → Only affects tests (no external consumers yet). All test helpers (`valid_role()`, `valid_task()` etc.) will need updating. Acceptable cost.
- **`clippy::pedantic` may surface many lints on first pass** → Fix rather than suppress where reasonable; suppress with explanation where not. One-time cost.
- **`serde(transparent)` on newtypes removes the wrapping layer in JSON** → Desired behaviour — IDs remain plain strings in YAML/JSON.

## Open Questions

None. All decisions above are resolved.
