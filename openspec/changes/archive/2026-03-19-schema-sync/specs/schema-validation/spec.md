## ADDED Requirements

### Requirement: Schemars annotations and validator checks serve different enforcement surfaces
Schemars annotations (e.g. `#[schemars(regex(...))]`, `#[schemars(length(min = N))]`) control what appears in the generated JSON schemas — they do NOT enforce constraints at the Rust type level. Validator functions SHALL continue to enforce all structural constraints at runtime, independent of what annotations exist on the type.

#### Scenario: Annotation does not replace runtime check
- **WHEN** a field carries a `#[schemars(regex(...))]` annotation
- **THEN** the corresponding `validate()` function still checks the pattern at runtime

#### Scenario: minItems annotation does not replace runtime check
- **WHEN** a field carries a `#[schemars(length(min = N))]` annotation
- **THEN** the corresponding `validate()` function still checks the minimum length at runtime

### Requirement: Semantic constraints remain in the validation layer
Constraints that cannot be expressed in JSON Schema SHALL remain in validator functions and continue to be tested via Rust unit tests.

#### Scenario: Referential integrity still validated
- **WHEN** a field references an entity id (role, hook, team, workflow)
- **THEN** the validator checks that id exists in `RepoContext`

#### Scenario: Cross-field constraint still validated
- **WHEN** a `ReviewStep.on_reject` references a step name
- **THEN** the validator checks that the referenced step appears earlier in the steps array

#### Scenario: State semantic constraint still validated
- **WHEN** a workflow or task has a states array
- **THEN** the validator checks at least one state has `semantic: complete` and at least one does not

#### Scenario: Cycle detection still validated
- **WHEN** a team references another team via `include` or `import`
- **THEN** the validator checks for cycles via `RepoContext.team_direct_refs`
