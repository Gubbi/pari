## 1. Dependencies and Scaffolding

- [x] 1.1 Add `thiserror = "1"` to `[dependencies]` in `Cargo.toml`
- [x] 1.2 Create `src/schema/ids.rs` (empty module) and declare it in `src/schema/mod.rs`

## 2. Newtype IDs — Kebab-case (Role, Team, Hook)

- [x] 2.1 Write tests for `RoleId`, `TeamId`, `HookId`: round-trip serialization as plain string, JSON schema contains kebab-case regex
- [x] 2.2 Implement `RoleId`, `TeamId`, `HookId` in `src/schema/ids.rs` with `#[serde(transparent)]` and `#[schemars(regex(...))]` on the inner field
- [x] 2.3 Update `Role`, `Team`, `Hook` entity structs to use the newtype for their `id` field
- [x] 2.4 Update all test helpers and inline tests for `Role`, `Team`, `Hook` to construct IDs via the newtype
- [x] 2.5 Run `cargo test` — all tests pass

## 3. Newtype IDs — CamelCase (Workflow, Task, Relay)

- [x] 3.1 Write tests for `WorkflowId`, `TaskId`, `RelayId`: round-trip serialization as plain string, JSON schema contains CamelCase regex
- [x] 3.2 Implement `WorkflowId`, `TaskId`, `RelayId` in `src/schema/ids.rs`
- [x] 3.3 Update `Workflow`, `SharedWorkflow`, `Task`, `Relay` entity structs to use the newtype for their `id` field
- [x] 3.4 Update all test helpers and inline tests for `Workflow`, `Task`, `Relay` to construct IDs via the newtype
- [x] 3.5 Run `cargo test` — all tests pass

## 4. Schema Regeneration

- [x] 4.1 Run `cargo xtask` to regenerate `schemas/` after ID type changes
- [x] 4.2 Verify regenerated schemas include the regex constraints from the newtypes (inspect `schemas/role.json`, `schemas/workflow.json`)
- [x] 4.3 Run `cargo test` — schema coherence tests pass

## 5. thiserror — ValidationError

- [x] 5.1 Write tests for `ValidationError`: `Display` output matches `"{message} at {path}"`, satisfies `dyn std::error::Error`
- [x] 5.2 Apply `#[derive(thiserror::Error)]` and `#[error(...)]` to `ValidationError` in `src/schema/validation.rs`
- [x] 5.3 Run `cargo test` — all validation tests pass

## 6. thiserror — SubstrateError

- [x] 6.1 Write tests for `SubstrateError`: `Display` output is human-readable, satisfies `dyn std::error::Error`
- [x] 6.2 Apply `#[derive(thiserror::Error)]` and `#[error(...)]` to `SubstrateError` in `src/substrate/`
- [x] 6.3 Run `cargo test` — all substrate tests pass

## 7. rustfmt

- [x] 7.1 Add `rustfmt.toml` at crate root with `edition = "2021"` (nightly-only import options omitted)
- [x] 7.2 Run `cargo fmt` across all source files
- [x] 7.3 Verify `cargo fmt --check` exits with code 0

## 8. clippy::pedantic

- [x] 8.1 Add `#![warn(clippy::pedantic)]` to `src/lib.rs`
- [x] 8.2 Run `cargo clippy -- -D warnings`; fix code or add targeted `#[allow(...)]` with explanatory comment for each lint
- [x] 8.3 Verify `cargo clippy -- -D warnings` exits with code 0

## 9. Module Doc Comments

- [x] 9.1 Add `//!` doc comment to every `.rs` file in `src/` describing the module's purpose
- [x] 9.2 Run `cargo doc` and verify no "missing documentation" warnings for public modules

## 10. cargo deny

- [x] 10.1 Create `deny.toml` at crate root with `[advisories]`, `[licenses]`, `[bans]`, `[sources]`
- [x] 10.2 Run `cargo deny check` and resolve any findings
- [x] 10.3 Verify `cargo deny check` exits with code 0

## 11. Final Verification

- [x] 11.1 Run `cargo test --all` — all tests pass
- [x] 11.2 Run `cargo fmt --check` — no diff
- [x] 11.3 Run `cargo clippy -- -D warnings` — clean
- [x] 11.4 Run `cargo deny check` — clean
