## Why

The codebase has grown to a point where error types lack standard `Display`/`Error` trait implementations, entity IDs have no type-level distinction, and there is no enforced formatting, linting, or dependency vetting. Establishing these conventions now — before the substrate and store layers expand — avoids retrofitting them under pressure.

## What Changes

- Add `thiserror` crate; apply `#[derive(thiserror::Error)]` to `ValidationError` and `SubstrateError` with `#[error(...)]` messages
- Introduce newtype wrappers for entity IDs: `RoleId`, `TeamId`, `HookId`, `WorkflowId` — each deriving `Serialize`, `Deserialize`, `JsonSchema`; update entity structs to use them
- Add `rustfmt.toml` with `edition = "2021"`, `imports_granularity = "Crate"`, `group_imports = "StdExternalCrate"`; reformat all source files
- Enable `clippy::pedantic` warnings crate-wide via `#![warn(clippy::pedantic)]` in `lib.rs`; fix or suppress all resulting lints
- Add `//!` module-level doc comments to all source modules describing their purpose
- Add `deny.toml` and integrate `cargo deny check` into the development workflow for license and advisory enforcement

## Capabilities

### New Capabilities

- `rust-conventions`: Formatting, linting, dependency auditing, typed IDs, standard error trait conformance, and module documentation as enforced project conventions

### Modified Capabilities

- `schema-validation`: `ValidationError` gains `Display` and `std::error::Error` via `thiserror`

## Impact

- `Cargo.toml`: adds `thiserror` dependency
- `src/schema/validation.rs`: `ValidationError` refactored to use `thiserror`
- `src/substrate/`: `SubstrateError` refactored to use `thiserror`
- `src/schema/types.rs` or new `src/schema/ids.rs`: newtype ID types introduced
- All entity structs (`Role`, `Team`, `Hook`, `Workflow`, `SharedWorkflow`): `id` field type changes from `String` to the corresponding newtype
- `schemas/`: regenerated via `cargo xtask` after type changes
- `lib.rs`: `#![warn(clippy::pedantic)]` added; clippy fixes applied throughout
- All `src/**/*.rs` modules: `//!` doc comment added at top of each file
- `rustfmt.toml`: new file at crate root
- `deny.toml`: new file at crate root
- No public API additions; ID newtypes are **BREAKING** for any downstream code constructing entity structs directly
