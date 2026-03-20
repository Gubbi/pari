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

---

### Requirement: RACI referential integrity
The system SHALL validate that all role_ids referenced in any RACI block (`responsible`, `accountable`, `consulted`, `informed`) exist as Role ids in the repository context.

#### Scenario: Valid RACI role references
- **WHEN** all role_ids in a RACI block exist in the repo
- **THEN** validation passes

#### Scenario: Unknown role in responsible
- **WHEN** `responsible` references a role_id not in the repo
- **THEN** validation fails with a path pointing to the offending field

#### Scenario: Unknown role in consulted list
- **WHEN** one entry in `consulted` does not exist as a Role id
- **THEN** validation fails with a path pointing to the offending entry

---

### Requirement: Hook invocation referential integrity
The system SHALL validate that all hook_ids referenced in any HooksMap (bare string or `hook` field in invocation objects) exist as Hook ids in the repository context.

#### Scenario: Valid hook reference
- **WHEN** a hook invocation references a hook_id that exists in the repo
- **THEN** validation passes

#### Scenario: Unknown hook_id
- **WHEN** a hook invocation references a hook_id not in the repo
- **THEN** validation fails with a path pointing to the offending invocation

---

### Requirement: Hook invocation input validation
The system SHALL validate hook invocation `with` maps against the Hook's declared `inputs`: all `required: true` inputs MUST be present in `with`, and no unknown keys are allowed.

#### Scenario: All required inputs provided
- **WHEN** a hook invocation's `with` contains all inputs where `required: true`
- **THEN** validation passes

#### Scenario: Missing required input
- **WHEN** a hook invocation omits a `required: true` input from `with`
- **THEN** validation fails indicating the missing input name

#### Scenario: Unknown key in with
- **WHEN** a hook invocation's `with` contains a key not declared in the Hook's `inputs`
- **THEN** validation fails indicating the unknown key

---

### Requirement: Relay delegates_to referential integrity
The system SHALL validate that a Relay's `delegates_to` references a Workflow id that exists in the `shared/` scope of the repository context.

#### Scenario: Valid shared workflow reference
- **WHEN** `delegates_to` names a workflow that exists in `shared/`
- **THEN** validation passes

#### Scenario: Unknown shared workflow
- **WHEN** `delegates_to` names a workflow not present in `shared/`
- **THEN** validation fails with path `delegates_to`

---

### Requirement: Relay state_map key integrity
The system SHALL validate that every key in a Relay's `state_map` exactly matches a state name defined in the referenced shared workflow's `states`.

#### Scenario: All state_map keys match
- **WHEN** every key in `state_map` is a state name in the shared workflow
- **THEN** validation passes

#### Scenario: state_map key not in shared workflow
- **WHEN** a `state_map` key does not match any state name in the shared workflow
- **THEN** validation fails with a path to the offending key

#### Scenario: Unmapped shared workflow states
- **WHEN** the shared workflow has states not covered by the Relay's `state_map`
- **THEN** validation passes (unmapped states are silently ignored)

---

### Requirement: Relay state_map complete semantic required
The system SHALL validate that a Relay's `state_map` contains at least one entry with `semantic: complete` and at least one entry without `semantic: complete`.

#### Scenario: Has complete and non-complete
- **WHEN** `state_map` has one entry with `semantic: complete` and another with no semantic
- **THEN** validation passes

#### Scenario: Missing complete semantic
- **WHEN** no entry in `state_map` has `semantic: complete`
- **THEN** validation fails

#### Scenario: All entries are complete
- **WHEN** every entry in `state_map` has `semantic: complete`
- **THEN** validation fails

---

### Requirement: Workflow and Task states semantic constraints
The system SHALL validate that any `states` array on a Workflow or Task contains at least one entry with `semantic: complete` and at least one entry without `semantic: complete`. For Workflows (only), if `steps` contain at least one ReviewStep, the `states` array SHALL also contain at least one entry with `semantic: reviewing`. The `reviewing` semantic is not applicable to Task states.

#### Scenario: States has complete and non-complete
- **WHEN** `states` has one entry with `semantic: complete` and at least one entry without it
- **THEN** validation passes

#### Scenario: Missing complete semantic
- **WHEN** no entry in `states` has `semantic: complete`
- **THEN** validation fails

#### Scenario: All states are complete
- **WHEN** every entry in `states` has `semantic: complete`
- **THEN** validation fails

#### Scenario: Workflow with ReviewStep missing reviewing semantic
- **WHEN** a Workflow has at least one ReviewStep in `steps` but no state with `semantic: reviewing`
- **THEN** validation fails

#### Scenario: Workflow with no ReviewStep does not require reviewing
- **WHEN** a Workflow has only WorkSteps and no state with `semantic: reviewing`
- **THEN** validation passes

---

### Requirement: ReviewStep name uniqueness within workflow
The system SHALL validate that all ReviewStep `name` values are unique within the same `steps` array.

#### Scenario: Unique review step names
- **WHEN** all ReviewSteps in a Workflow's steps have distinct names
- **THEN** validation passes

#### Scenario: Duplicate review step name
- **WHEN** two ReviewSteps in the same Workflow share the same `name`
- **THEN** validation fails indicating the duplicate

---

### Requirement: ReviewStep on_reject references earlier step
The system SHALL validate that a ReviewStep's `on_reject` value names a step that appears before the ReviewStep in the same `steps` array.

#### Scenario: on_reject references earlier step
- **WHEN** `on_reject` names a step at a lower index in the steps array
- **THEN** validation passes

#### Scenario: on_reject references later step
- **WHEN** `on_reject` names a step at a higher index
- **THEN** validation fails

#### Scenario: on_reject references unknown step
- **WHEN** `on_reject` names a step not present in the steps array
- **THEN** validation fails

---

### Requirement: WorkStep depends_on references valid steps
The system SHALL validate that all names listed in a WorkStep's `depends_on` exist as step names in the same `steps` array.

#### Scenario: All depends_on names exist
- **WHEN** every name in `depends_on` is a step name in the same workflow
- **THEN** validation passes

#### Scenario: Unknown step in depends_on
- **WHEN** `depends_on` contains a name not in the steps array
- **THEN** validation fails

---

### Requirement: Team member handle uniqueness
The system SHALL validate that all member `handle` values are unique within a Team.

#### Scenario: Unique handles
- **WHEN** all members in a Team have distinct handles
- **THEN** validation passes

#### Scenario: Duplicate handle
- **WHEN** two members share the same handle
- **THEN** validation fails

---

### Requirement: Team include and import referential integrity
The system SHALL validate that all team_ids in a Team's `include` map keys and `import` list exist in the repository context.

#### Scenario: Valid team references
- **WHEN** all team_ids in `include` and `import` exist in the repo
- **THEN** validation passes

#### Scenario: Unknown team in include
- **WHEN** `include` references a team_id not in the repo
- **THEN** validation fails

#### Scenario: Unknown team in import
- **WHEN** `import` lists a team_id not in the repo
- **THEN** validation fails

---

### Requirement: Team include role referential integrity
The system SHALL validate that all role_id values in a Team's `include` map exist in the repository context.

#### Scenario: Valid role in include
- **WHEN** all role_id values in `include` exist in the repo
- **THEN** validation passes

#### Scenario: Unknown role in include
- **WHEN** an `include` value references a role_id not in the repo
- **THEN** validation fails

---

### Requirement: Team member role referential integrity
The system SHALL validate that each member's `role` in a Team exists in the repository context.

#### Scenario: Valid member role
- **WHEN** a member's `role` exists in the repo
- **THEN** validation passes

#### Scenario: Unknown member role
- **WHEN** a member's `role` does not exist in the repo
- **THEN** validation fails

---

### Requirement: Team include and import conflict resolution
The system SHALL document that when the same handle appears via multiple sources, the precedence order is: direct `members` entries win over `import`, and `import` wins over `include`. When the same handle appears in multiple `import` entries, the last `import` entry in the list wins.

#### Scenario: Direct member overrides import
- **WHEN** a handle appears in both `members` and an `import`-ed team
- **THEN** the direct `members` entry takes precedence

#### Scenario: Import overrides include
- **WHEN** a handle appears in both an `import`-ed team and an `include`-d team
- **THEN** the `import` entry takes precedence

#### Scenario: Last import wins
- **WHEN** the same handle appears in two `import`-ed teams
- **THEN** the entry from the later team in the `import` list takes precedence

---

### Requirement: Team no circular include or import
The system SHALL validate that a Team does not form a circular chain through its `include` or `import` references. Since `RepoContext` contains only already-validated teams (none of which can reference the incoming team), the check reduces to ensuring the incoming team does not appear in the reachable chain of any team it references.

#### Scenario: No cycle
- **WHEN** none of the teams reachable from the incoming team's `include` or `import` chains reference the incoming team
- **THEN** validation passes

#### Scenario: Self-reference via include
- **WHEN** a Team's `include` directly names itself
- **THEN** validation fails

#### Scenario: Transitive cycle
- **WHEN** a team A `import`s team B, and team B `import`s the incoming team
- **THEN** validation fails

---

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

---

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

### Requirement: Shared workflow step type constraint
The system SHALL validate that no step in a `SharedWorkflow` embeds a `Relay` definition. Since `SharedWorkStepDefinition` excludes `Relay` at the type level, this constraint is enforced structurally; any `delegates_to` field on a step definition in a shared workflow is a structural validation error.

#### Scenario: Task step in shared workflow passes
- **WHEN** a SharedWorkflow step embeds a Task definition
- **THEN** validation passes

#### Scenario: Relay step in shared workflow fails
- **WHEN** a SharedWorkflow step embeds a definition with `delegates_to`
- **THEN** validation fails indicating Relay is not permitted in shared workflows

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

### Requirement: WorkStep depends_on references valid steps
The system SHALL validate that all names listed in a WorkStep's `depends_on` exist as step names in the same `steps` array. Step names are now derived from the `id` field of each step's embedded `WorkStepDefinition` (not a standalone `id` on WorkStep itself). ReviewStep names come from `ReviewStep.id` as before.

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
