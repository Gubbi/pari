## ADDED Requirements

### Requirement: Newtype wrappers for all entity IDs
The system SHALL define newtype wrappers for all entity ID fields in `src/schema/ids.rs`. Each newtype SHALL wrap `String` and derive `Serialize`, `Deserialize`, and `JsonSchema`. Each newtype SHALL be `#[serde(transparent)]` so it serializes as a plain string. Format constraints (regex patterns) SHALL be declared on the inner field via `#[schemars(regex(...))]`.

Newtypes: `RoleId` (kebab-case), `TeamId` (kebab-case), `HookId` (kebab-case), `WorkflowId` (CamelCase), `TaskId` (CamelCase), `RelayId` (CamelCase).

Entity structs SHALL use the corresponding newtype for their `id` field instead of `String`.

#### Scenario: RoleId serializes as plain string
- **WHEN** a `Role` with `id: RoleId("eng-lead".into())` is serialized to JSON
- **THEN** the output contains `"id": "eng-lead"` (not a wrapped object)

#### Scenario: WorkflowId rejects kebab-case at schema level
- **WHEN** the generated JSON schema for `Workflow` is inspected
- **THEN** the `id` field's schema includes the CamelCase regex pattern `^[A-Z][A-Za-z0-9]*$`

#### Scenario: TaskId and RelayId are defined
- **WHEN** the `ids` module is inspected
- **THEN** `TaskId` and `RelayId` exist and are used in their respective embedded entity structs

---

### Requirement: rustfmt enforced via rustfmt.toml
The project SHALL include a `rustfmt.toml` at the crate root with:
- `edition = "2021"`
- `imports_granularity = "Crate"`
- `group_imports = "StdExternalCrate"`

All source files SHALL be formatted under this configuration. `cargo fmt --check` SHALL pass with no diff.

#### Scenario: Import ordering is enforced
- **WHEN** a source file has interleaved std, external, and crate imports
- **THEN** `cargo fmt` groups them into three distinct blocks: std → external → crate

#### Scenario: fmt check passes on all source files
- **WHEN** `cargo fmt --check` is run
- **THEN** it exits with code 0 and reports no formatting differences

---

### Requirement: clippy::pedantic enabled crate-wide
`src/lib.rs` SHALL include `#![warn(clippy::pedantic)]`. All resulting lint warnings SHALL be resolved — either by fixing the code or by a targeted `#[allow(...)]` with a comment explaining the suppression. No warnings SHALL remain unaddressed.

#### Scenario: cargo clippy passes clean
- **WHEN** `cargo clippy -- -D warnings` is run
- **THEN** it exits with code 0

#### Scenario: Suppressions are documented
- **WHEN** a `#[allow(clippy::...)]` annotation is present
- **THEN** it is accompanied by a comment explaining why suppression is appropriate

---

### Requirement: Module-level doc comments on all source modules
Every `.rs` file in `src/` SHALL begin with a `//!` doc comment describing the module's purpose. The comment SHALL be a concise statement of what the module contains or does — sufficient for `cargo doc` to produce useful output.

#### Scenario: cargo doc generates output for all modules
- **WHEN** `cargo doc` is run
- **THEN** every public module has a non-empty module-level description in the generated docs

---

### Requirement: cargo deny configured for dependency auditing
The project SHALL include a `deny.toml` at the crate root configured with:
- `[advisories]`: deny vulnerable and unsound crates
- `[licenses]`: allow MIT, Apache-2.0, BSD-2-Clause, BSD-3-Clause, ISC, Unicode-3.0; deny unlicensed
- `[bans]`: warn on multiple versions of the same crate
- `[sources]`: allow crates.io only

`cargo deny check` SHALL pass with no errors against the current dependency tree.

#### Scenario: cargo deny check passes
- **WHEN** `cargo deny check` is run against the current Cargo.lock
- **THEN** it exits with code 0 with no denied advisories or licenses
