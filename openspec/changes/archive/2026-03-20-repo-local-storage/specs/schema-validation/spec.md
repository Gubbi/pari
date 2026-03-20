## ADDED Requirements

### Requirement: Extension key format validation
The system SHALL validate that every key in an entity's `extensions` map matches the pattern `^x-`. Any key that does not carry the `x-` prefix SHALL produce a `ValidationError`. This check runs during structural validation (Phase 1) for every entity type, including entities embedded within `WorkStepDefinition`.

#### Scenario: Valid x- key passes
- **WHEN** an entity's extensions contain only `x-` prefixed keys
- **THEN** validation passes

#### Scenario: Non-prefixed key fails
- **WHEN** an entity's extensions contain a key `team` (no `x-` prefix)
- **THEN** validation fails with a path pointing to the offending key

#### Scenario: Extension keys on embedded Task validated
- **WHEN** a Task embedded in a WorkStep has an extension key without `x-` prefix
- **THEN** validation fails with a path that includes the step index (e.g., `steps[1].definition.extensions`)

---

### Requirement: Shared workflow step type constraint
The system SHALL validate that no step in a `SharedWorkflow` embeds a `Relay` definition. Since `SharedWorkStepDefinition` excludes `Relay` at the type level, this constraint is enforced structurally; any `delegates_to` field on a step definition in a shared workflow is a structural validation error.

#### Scenario: Task step in shared workflow passes
- **WHEN** a SharedWorkflow step embeds a Task definition
- **THEN** validation passes

#### Scenario: Relay step in shared workflow fails
- **WHEN** a SharedWorkflow step embeds a definition with `delegates_to`
- **THEN** validation fails indicating Relay is not permitted in shared workflows

---

### Requirement: Two-phase validation protocol
All entity validators SHALL split their logic into two phases that execute in strict order:

- **Phase 1 — Structural validation**: Validates the shape, type constraints, format rules, and internal consistency of the entity and all its embedded children. This phase collects errors without consulting `RepoContext`. Structural checks include: required field presence, `id` format (kebab-case / CamelCase), `extensions` key prefix, array minimum lengths, and embedded entity structural errors (prefixed with their step path). Extension key format validation (`validate_extensions`) runs in Phase 1.
- **Phase 2 — Semantic validation**: Validates referential integrity and cross-entity constraints using `RepoContext`. This phase runs only when Phase 1 produces zero errors. Semantic checks include: `depends_on` step name lookup, `delegates_to` SharedWorkflow lookup, hook id lookup, and role id lookup in RACI fields.

If Phase 1 produces any errors, Phase 2 is skipped entirely and only Phase 1 errors are returned. This ensures that semantic checks never run against a structurally incomplete entity, which would produce misleading errors.

#### Scenario: Phase 1 runs before Phase 2
- **WHEN** a Workflow validator is invoked
- **THEN** structural errors are collected first before any `RepoContext` lookups are performed

#### Scenario: Phase 2 skipped when Phase 1 has errors
- **WHEN** the entity tree has structural errors (e.g., a Task with missing `purpose`)
- **THEN** semantic validation is not run and only structural errors are returned

#### Scenario: Phase 2 runs when Phase 1 is clean
- **WHEN** the entity tree is structurally valid
- **THEN** semantic validation runs and referential errors (e.g., unknown `delegates_to`) are reported

---

### Requirement: Embedded entity validation is composed through the parent
The system SHALL validate all embedded entities (Task, Relay, inline Workflow) within a parent Workflow's steps as part of validating that parent. The parent calls each embedded entity's validator and prefixes all returned errors with the embedding step's path (e.g., `steps[2].definition`). This delegation preserves the two-phase ordering: the parent calls `validate_structure_tree` on all children before calling `validate_semantic_tree` on any of them.

#### Scenario: Embedded Task structural error surfaces with step path
- **WHEN** a Task embedded at `steps[1]` has an invalid `id` format
- **THEN** validation reports the error at `steps[1].definition.id`

#### Scenario: Embedded Relay semantic error surfaces with step path
- **WHEN** a Relay embedded at `steps[0]` references an unknown shared workflow in `delegates_to`
- **THEN** validation reports the error at `steps[0].definition.delegates_to`

#### Scenario: Inline Workflow errors surface recursively
- **WHEN** an inline Workflow embedded at `steps[2]` has a WorkStep whose embedded Task has an empty `criteria`
- **THEN** validation reports the error at `steps[2].definition.steps[0].definition.criteria`

#### Scenario: Semantic validation skipped when structural errors present anywhere in tree
- **WHEN** any node in the entity tree has structural errors
- **THEN** semantic validation is skipped for the entire tree, not just the failing node

---

## MODIFIED Requirements

### Requirement: WorkStep depends_on references valid steps
The system SHALL validate that all names listed in a WorkStep's `depends_on` exist as step names in the same `steps` array. Step names are now derived from the `id` field of each step's embedded `WorkStepDefinition` (not a standalone `id` on WorkStep itself). ReviewStep names come from `ReviewStep.name` as before.

#### Scenario: All depends_on names exist
- **WHEN** every name in `depends_on` matches the `id` of another embedded definition in the same workflow
- **THEN** validation passes

#### Scenario: Unknown step in depends_on
- **WHEN** `depends_on` contains a name not matching any embedded definition id in the steps array
- **THEN** validation fails

---

### Requirement: Relay delegates_to referential integrity
The system SHALL validate that a Relay's `delegates_to` references the `id` of a `SharedWorkflow` present in `RepoContext.shared_workflows` (a typed `Vec<SharedWorkflow>`, not a `Vec<String>`). The lookup is performed against the `id` field of each `SharedWorkflow` in the context.

#### Scenario: Valid shared workflow reference
- **WHEN** `delegates_to` names a SharedWorkflow id present in `RepoContext.shared_workflows`
- **THEN** validation passes

#### Scenario: Unknown shared workflow
- **WHEN** `delegates_to` names an id not present in `RepoContext.shared_workflows`
- **THEN** validation fails with path `delegates_to`
