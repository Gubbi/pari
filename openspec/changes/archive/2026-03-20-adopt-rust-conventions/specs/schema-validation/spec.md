## MODIFIED Requirements

### Requirement: Structured validation errors
The system SHALL represent each validation failure as a `ValidationError` with a `path` (dot-notation string identifying the field, e.g., `steps[2].on_reject`) and a `message` (human-readable description). Validation SHALL collect all errors in a single pass rather than stopping at the first failure.

`ValidationError` SHALL implement `std::fmt::Display` and `std::error::Error`. The `Display` output SHALL be the message followed by the path, in the form: `"{message} at {path}"`. These implementations SHALL be derived via `thiserror`.

#### Scenario: Multiple errors collected
- **WHEN** an entity has two distinct validation failures
- **THEN** both errors are returned in the result

#### Scenario: Error path is specific
- **WHEN** validation fails on a nested field
- **THEN** the `path` in the error identifies the exact field location

#### Scenario: ValidationError displays as human-readable string
- **WHEN** a `ValidationError` with path `"id"` and message `"id must be kebab-case, got 'Foo'"` is formatted with `Display`
- **THEN** the output is `"id must be kebab-case, got 'Foo' at id"`

#### Scenario: ValidationError implements std::error::Error
- **WHEN** a `ValidationError` is used where `dyn std::error::Error` is expected
- **THEN** it satisfies the trait bound without additional wrapping
