## ADDED Requirements

### Requirement: Task and Relay are embedded-only entities
`Task` and `Relay` SHALL NOT exist as top-level workflow entities or have standalone schema files generated for them. They are exclusively defined as variants of `WorkStepDefinition` and exist only when embedded within a `WorkStep.definition`. Removing them as top-level entities is a deliberate design choice: a Task or Relay without a parent Workflow context has no meaning in the runtime model. The xtask schema generator SHALL NOT emit `schemas/task.json` or `schemas/relay.json`; any previously generated files at those paths SHALL be deleted.

#### Scenario: No standalone Task schema file
- **WHEN** the xtask schema generator runs
- **THEN** no `schemas/task.json` file is produced

#### Scenario: No standalone Relay schema file
- **WHEN** the xtask schema generator runs
- **THEN** no `schemas/relay.json` file is produced

#### Scenario: Task accessible via WorkStepDefinition
- **WHEN** a WorkStep's `definition` object contains an `artifact` field
- **THEN** it deserializes as a `Task` embedded within the step

#### Scenario: Relay accessible via WorkStepDefinition
- **WHEN** a WorkStep's `definition` object contains a `delegates_to` field
- **THEN** it deserializes as a `Relay` embedded within the step

---

### Requirement: Extensions type schema
The system SHALL define an `Extensions` newtype (`HashMap<String, serde_json::Value>`) that carries user-defined fields. Its JSON Schema representation SHALL contribute `patternProperties: { "^x-": {} }` to the containing entity schema so that `x-`-prefixed fields are explicitly allowed.

#### Scenario: Extensions serializes x- prefixed keys
- **WHEN** an entity instance has `extensions` containing `{ "x-team": "platform" }`
- **THEN** the field round-trips through serde without loss

#### Scenario: schemars emits patternProperties for Extensions
- **WHEN** the JSON Schema for any entity type is generated
- **THEN** the schema includes `patternProperties: { "^x-": {} }` and `additionalProperties: false`

---

### Requirement: WorkStepDefinition schema
The system SHALL define `WorkStepDefinition` as an untagged enum of three variants: `Task`, `Relay`, and `Box<Workflow>`. Discrimination between variants is by the presence of their respective distinguishing required fields: `artifact` (Task), `delegates_to` (Relay), `steps` (Workflow).

#### Scenario: Task variant selected when artifact present
- **WHEN** a WorkStepDefinition object has an `artifact` field
- **THEN** it deserializes as the `Task` variant

#### Scenario: Relay variant selected when delegates_to present
- **WHEN** a WorkStepDefinition object has a `delegates_to` field
- **THEN** it deserializes as the `Relay` variant

#### Scenario: Workflow variant selected when steps present
- **WHEN** a WorkStepDefinition object has a `steps` field
- **THEN** it deserializes as the inline `Workflow` variant

---

### Requirement: SharedWorkStepDefinition schema
The system SHALL define `SharedWorkStepDefinition` as an untagged enum of two variants: `Task` and `Box<SharedWorkflow>`. Relay is excluded — shared workflows may not embed Relay steps.

#### Scenario: Task variant in shared workflow
- **WHEN** a SharedWorkStepDefinition object has an `artifact` field
- **THEN** it deserializes as the `Task` variant

#### Scenario: SharedWorkflow variant in shared workflow
- **WHEN** a SharedWorkStepDefinition object has a `steps` field
- **THEN** it deserializes as the `SharedWorkflow` variant

---

### Requirement: SharedWorkflow schema
The system SHALL define `SharedWorkflow` as `WorkflowDef<SharedStep>` where `SharedStep` is an untagged enum of `SharedWorkStep | ReviewStep`. `SharedWorkflow` has the same fields as `Workflow` and the same required field constraints, except its `steps` contain only `SharedWorkStep` or `ReviewStep` items.

#### Scenario: Valid shared workflow
- **WHEN** a SharedWorkflow has all required fields and steps containing only Task or ReviewStep definitions
- **THEN** it is schema-valid

#### Scenario: Relay step in shared workflow is schema-invalid
- **WHEN** a step in a SharedWorkflow has `delegates_to`
- **THEN** it is schema-invalid (Relay is not a valid SharedWorkStepDefinition variant)

---

## MODIFIED Requirements

### Requirement: WorkStep embedded type schema
The system SHALL define a JSON Schema for `WorkStep` with fields: `depends_on` (array of step name strings, optional) and `definition` (WorkStepDefinition, required). `WorkStep` has no independent identity field — its identity is the `id` of its embedded `WorkStepDefinition`.

#### Scenario: WorkStep with Task definition
- **WHEN** a WorkStep has a `definition` that is a valid Task
- **THEN** it is schema-valid

#### Scenario: WorkStep with depends_on
- **WHEN** a WorkStep has `depends_on` listing step names
- **THEN** it is schema-valid

#### Scenario: WorkStep without depends_on
- **WHEN** a WorkStep has only `definition`
- **THEN** it is schema-valid

#### Scenario: WorkStep without definition is invalid
- **WHEN** a WorkStep omits `definition`
- **THEN** it is schema-invalid

---

### Requirement: Role schema
The system SHALL define a JSON Schema for the `Role` entity with fields: `id` (kebab-case string, required), `name` (string, required), `purpose` (string, required), `traits` (array of strings, optional), and user extensions (`x-` prefixed keys, optional). Non-`x-` unknown keys SHALL be schema-invalid.

#### Scenario: Valid role
- **WHEN** a Role has `id` in kebab-case, `name`, and `purpose`
- **THEN** it is schema-valid

#### Scenario: Traits optional
- **WHEN** a Role omits `traits`
- **THEN** it is schema-valid

#### Scenario: Missing purpose
- **WHEN** a Role omits `purpose`
- **THEN** it is schema-invalid

#### Scenario: x- extension key allowed
- **WHEN** a Role has an `x-hiring` field
- **THEN** it is schema-valid

#### Scenario: Unknown non-x key rejected
- **WHEN** a Role has an unknown field `hiring` (no `x-` prefix)
- **THEN** it is schema-invalid

---

### Requirement: Hook schema
The system SHALL define a JSON Schema for the `Hook` entity with fields: `id` (CamelCase string, required), `name` (string, required), `description` (string, required), `instructions` (array of strings, required, min 1), `inputs` (array of input objects, optional), and user extensions (`x-` prefixed keys, optional). Non-`x-` unknown keys SHALL be schema-invalid.

#### Scenario: Valid hook with inputs
- **WHEN** a Hook has `id`, `name`, `description`, `instructions` with at least one item, and `inputs`
- **THEN** it is schema-valid

#### Scenario: Minimal hook
- **WHEN** a Hook has `id`, `name`, `description`, and `instructions` with at least one item
- **THEN** it is schema-valid

#### Scenario: Empty instructions
- **WHEN** a Hook has `instructions` as an empty array
- **THEN** it is schema-invalid

#### Scenario: Missing instructions
- **WHEN** a Hook omits `instructions`
- **THEN** it is schema-invalid

#### Scenario: x- extension key allowed
- **WHEN** a Hook has an `x-version` field
- **THEN** it is schema-valid

---

### Requirement: Team schema
The system SHALL define a JSON Schema for the `Team` entity with fields: `id` (kebab-case string, required), `name` (string, required), `description` (string, optional), `members` (array of member objects, optional), `include` (map of team_id to role_id, optional), `import` (array of team_ids, optional), and user extensions (`x-` prefixed keys, optional). Non-`x-` unknown keys SHALL be schema-invalid.

#### Scenario: Valid team with members
- **WHEN** a Team has `id` in kebab-case, `name`, and members each with `handle` and `role`
- **THEN** it is schema-valid

#### Scenario: Handle allows dot
- **WHEN** a member handle is `@alice.smith`
- **THEN** it is schema-valid

#### Scenario: Handle format violation
- **WHEN** a member handle does not start with `@`
- **THEN** it is schema-invalid

#### Scenario: Team with include and import
- **WHEN** a Team has `include` mapping team_ids to role_ids and `import` listing team_ids
- **THEN** it is schema-valid

#### Scenario: Empty team
- **WHEN** a Team has `id` and `name` and no members, include, or import
- **THEN** it is schema-valid

#### Scenario: x- extension key allowed
- **WHEN** a Team has an `x-department` field
- **THEN** it is schema-valid

---

### Requirement: Workflow schema
The system SHALL define a JSON Schema for the `Workflow` entity with fields: `id` (CamelCase, required), `name` (string, required), `description` (string, optional), `purpose` (string, required), `accountability` (RACI, required), `steps` (array of WorkStep or ReviewStep, required, min 1), `states` (array of Workflow state entries, required, min 2), `hooks` (HooksMap, optional), `guidance` (string, optional), and user extensions (`x-` prefixed keys, optional). Each WorkStep now embeds a WorkStepDefinition (Task, Relay, or inline Workflow) rather than referencing by id.

#### Scenario: Valid workflow
- **WHEN** a Workflow has all required fields, at least one step with an embedded definition, and at least two states
- **THEN** it is schema-valid

#### Scenario: Missing purpose
- **WHEN** a Workflow omits `purpose`
- **THEN** it is schema-invalid

#### Scenario: Missing accountability
- **WHEN** a Workflow omits `accountability`
- **THEN** it is schema-invalid

#### Scenario: Empty steps
- **WHEN** a Workflow has `steps` as an empty array
- **THEN** it is schema-invalid

#### Scenario: x- extension key allowed
- **WHEN** a Workflow has an `x-owner` field
- **THEN** it is schema-valid

---

### Requirement: Task schema
The system SHALL define a JSON Schema for the `Task` entity with fields: `id` (CamelCase, required), `name` (string, required), `description` (string, optional), `purpose` (string, required), `instructions` (array of strings, required, min 1), `criteria` (array of strings, required, min 1), `accountability` (RACI, optional), `artifact` (object with `name` string required and `template` string optional, required), `states` (array of Task state entries, required, min 2), `hooks` (HooksMap, optional), `guidance` (string, optional), and user extensions (`x-` prefixed keys, optional). Task exists only as an embedded `WorkStepDefinition` variant — no standalone schema file is generated for it.

#### Scenario: Valid task
- **WHEN** a Task has all required fields with non-empty `instructions` and `criteria`
- **THEN** it is schema-valid

#### Scenario: Missing purpose
- **WHEN** a Task omits `purpose`
- **THEN** it is schema-invalid

#### Scenario: Empty criteria
- **WHEN** a Task has `criteria` as an empty array
- **THEN** it is schema-invalid

#### Scenario: x- extension key allowed
- **WHEN** a Task has an `x-estimate` field
- **THEN** it is schema-valid

---

### Requirement: Relay schema
The system SHALL define a JSON Schema for the `Relay` entity with fields: `id` (CamelCase, required), `name` (string, required), `description` (string, optional), `purpose` (string, required), `accountability` (RACI, optional), `delegates_to` (string, required), `briefing` (string, optional), `debriefing` (string, optional), `state_map` (map of string to StateMapEntry, required, min 1), `hooks` (HooksMap, optional), `guidance` (string, optional), and user extensions (`x-` prefixed keys, optional). Relay exists only as an embedded `WorkStepDefinition` variant — no standalone schema file is generated for it.

#### Scenario: Valid relay
- **WHEN** a Relay has all required fields and at least one `state_map` entry
- **THEN** it is schema-valid

#### Scenario: Missing delegates_to
- **WHEN** a Relay omits `delegates_to`
- **THEN** it is schema-invalid

#### Scenario: Missing purpose
- **WHEN** a Relay omits `purpose`
- **THEN** it is schema-invalid

#### Scenario: x- extension key allowed
- **WHEN** a Relay has an `x-sla` field
- **THEN** it is schema-valid
