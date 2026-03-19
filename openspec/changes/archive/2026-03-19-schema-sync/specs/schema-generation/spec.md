## ADDED Requirements

### Requirement: Rust types are the source of truth for JSON schemas
All entity and shared types SHALL derive `JsonSchema` (via `schemars`), `Serialize`, and `Deserialize` (via `serde`). The `schemas/*.json` files are generated artifacts — they SHALL NOT be edited by hand.

#### Scenario: Type derives produce valid schema
- **WHEN** `schemars::schema_for!` is called on any entity type
- **THEN** it produces a valid JSON Schema document without panicking

#### Scenario: Generated schemas are committed
- **WHEN** `cargo xtask generate-schemas` is run
- **THEN** `schemas/*.json` files are written to disk and reflect the current type definitions

### Requirement: Structural constraints are encoded as type annotations
Constrained fields SHALL carry schemars annotations so the generated schema matches the hand-written schema it replaces.

#### Scenario: Id pattern annotation
- **WHEN** a type has an id field with a kebab-case or CamelCase constraint
- **THEN** the generated schema for that field includes the correct `pattern` value

#### Scenario: minItems annotation
- **WHEN** a field is a `Vec<T>` with a minimum item constraint
- **THEN** the generated schema for that field includes the correct `minItems` value

### Requirement: CI enforces schemas are not stale
CI SHALL run `cargo xtask generate-schemas` and fail if the output differs from the committed `schemas/*.json` files.

#### Scenario: Types changed without regenerating schemas
- **WHEN** a Rust type is modified but `cargo xtask generate-schemas` is not run
- **THEN** CI detects a diff and fails the build

#### Scenario: Schemas are up to date
- **WHEN** `cargo xtask generate-schemas` has been run after the latest type changes
- **THEN** CI passes with no diff

### Requirement: Schema coherence is verified in tests
A test SHALL generate schemas at test time and assert key constraints are present, catching annotation typos that compile successfully but produce wrong schemas.

#### Scenario: Pattern constraint present on id fields
- **WHEN** the schema coherence test runs
- **THEN** the generated schema for each entity's id field contains the expected `pattern`

#### Scenario: minItems constraint present on bounded arrays
- **WHEN** the schema coherence test runs
- **THEN** the generated schema for each bounded array field contains the expected `minItems`
