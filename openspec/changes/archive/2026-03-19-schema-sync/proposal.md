## Why

The JSON schemas in `schemas/` and the Rust types in `src/schema/` are maintained independently with no automated sync mechanism. Any field addition, pattern change, or constraint update on one side can silently diverge from the other, with no test or tooling to catch it.

## What Changes

- Add `serde` and `schemars` as production dependencies
- Derive `Serialize`, `Deserialize`, and `JsonSchema` on all entity and shared types
- Add `#[schemars(regex(...))]` and `#[schemars(length(min = N))]` annotations on constrained fields
- **BREAKING**: Remove hand-maintained `schemas/*.json` files — replaced by generated output
- Add `cargo xtask generate-schemas` command that generates `schemas/*.json` from Rust types
- Remove `tests/json_schema_validation.rs` — redundant once Rust types are the source of truth
- Shrink validation layer to semantic-only checks (referential integrity, cross-field constraints, state semantics)

## Capabilities

### New Capabilities

- `schema-generation`: Generating and committing JSON schema files from Rust types via schemars and an xtask

### Modified Capabilities

- `schema-validation`: Validation layer narrows to semantic constraints only — pattern and minItems checks move into type annotations, not validator functions

## Impact

- `Cargo.toml`: `serde` (with derive feature) and `schemars` added as production dependencies
- All types in `src/schema/types.rs` and `src/schema/entities/*.rs`: new derives and field annotations
- `src/schema/validation.rs`: `is_kebab_case`, `is_camel_case` checks removed from validators where now encoded in type annotations
- `tests/json_schema_validation.rs`: deleted
- `schemas/*.json`: become generated artifacts, regenerated via xtask
- New: `xtask/` crate with `generate-schemas` binary
