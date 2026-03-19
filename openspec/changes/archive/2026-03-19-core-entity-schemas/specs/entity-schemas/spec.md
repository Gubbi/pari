## ADDED Requirements

### Requirement: Role schema
The system SHALL define a JSON Schema for the `Role` entity with fields: `id` (kebab-case string, required), `name` (string, required), `purpose` (string, required), `traits` (array of strings, optional).

#### Scenario: Valid role
- **WHEN** a Role has `id` in kebab-case, `name`, and `purpose`
- **THEN** it is schema-valid

#### Scenario: Traits optional
- **WHEN** a Role omits `traits`
- **THEN** it is schema-valid

#### Scenario: Missing purpose
- **WHEN** a Role omits `purpose`
- **THEN** it is schema-invalid

---

### Requirement: Hook schema
The system SHALL define a JSON Schema for the `Hook` entity with fields: `id` (CamelCase string, required), `name` (string, required), `description` (string, required), `instructions` (array of strings, required, min 1), `inputs` (array of input objects, optional). Each input object SHALL have `name` (string, required), `description` (string, required), `required` (boolean, required).

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

---

### Requirement: RACI embedded type schema
The system SHALL define a JSON Schema for `RACI` with fields: `responsible` (role_id string, required), `accountable` (role_id string, required), `consulted` (array of role_id strings, required, may be empty), `informed` (array of role_id strings, required, may be empty).

#### Scenario: Valid RACI with empty lists
- **WHEN** RACI has `responsible`, `accountable`, and empty `consulted` and `informed`
- **THEN** it is schema-valid

#### Scenario: Missing accountable
- **WHEN** RACI omits `accountable`
- **THEN** it is schema-invalid

#### Scenario: Missing consulted
- **WHEN** RACI omits `consulted`
- **THEN** it is schema-invalid

---

### Requirement: HookInvocation embedded type schema
The system SHALL define a JSON Schema for `HookInvocation` as a union: either (a) a bare hook_id string, or (b) an object with `hook` (string, required) and `with` (map of string to string, optional).

#### Scenario: Bare hook_id
- **WHEN** a hook invocation is a string
- **THEN** it is a valid HookInvocation

#### Scenario: Invocation object with inputs
- **WHEN** a hook invocation is `{ "hook": "UpdateJiraStatus", "with": { "status": "Done" } }`
- **THEN** it is a valid HookInvocation

#### Scenario: Invocation object without with
- **WHEN** a hook invocation is `{ "hook": "UpdateJiraStatus" }`
- **THEN** it is a valid HookInvocation

#### Scenario: Invocation object missing hook
- **WHEN** a hook invocation object omits `hook`
- **THEN** it is schema-invalid

---

### Requirement: HooksMap embedded type schema
The system SHALL define a JSON Schema for `HooksMap`: a map whose keys are lifecycle point name strings and whose values are a single `HookInvocation` or an array of `HookInvocation` items.

#### Scenario: Single invocation at a lifecycle point
- **WHEN** a HooksMap is `{ "after": "NotifySlack" }`
- **THEN** it is schema-valid

#### Scenario: Multiple invocations at a lifecycle point
- **WHEN** a HooksMap is `{ "after": ["NotifySlack", { "hook": "UpdateJiraStatus", "with": { "status": "Done" } }] }`
- **THEN** it is schema-valid

---

### Requirement: Team schema
The system SHALL define a JSON Schema for the `Team` entity with fields: `id` (kebab-case string, required), `name` (string, required), `description` (string, optional), `members` (array of member objects, optional), `include` (map of team_id to role_id, optional), `import` (array of team_ids, optional). Each member SHALL have `handle` (string matching `@[a-z0-9._-]+`, required) and `role` (role_id string, required).

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

---

### Requirement: WorkStep embedded type schema
The system SHALL define a JSON Schema for `WorkStep` with fields: `name` (CamelCase string, required) and `depends_on` (array of step name strings, optional).

#### Scenario: Step with depends_on
- **WHEN** a WorkStep has `name` and `depends_on` listing step names
- **THEN** it is schema-valid

#### Scenario: Step without depends_on
- **WHEN** a WorkStep has only `name`
- **THEN** it is schema-valid

---

### Requirement: ReviewStep embedded type schema
The system SHALL define a JSON Schema for `ReviewStep` with fields: `name` (CamelCase string, required), `approver` (role_id string, required), `on_reject` (step name string, required).

#### Scenario: Valid review step
- **WHEN** a ReviewStep has `name`, `approver`, and `on_reject`
- **THEN** it is schema-valid

#### Scenario: Missing on_reject
- **WHEN** a ReviewStep omits `on_reject`
- **THEN** it is schema-invalid

---

### Requirement: Workflow state entry schema
The system SHALL define a JSON Schema for a Workflow state entry with fields: `name` (string, required), `description` (string, required), and `semantic` (string from closed set `[reviewing, complete, blocked, failed]`, optional).

#### Scenario: State with reviewing semantic
- **WHEN** a Workflow state entry has `semantic: reviewing`
- **THEN** it is schema-valid

#### Scenario: State without semantic
- **WHEN** a Workflow state entry has `name` and `description` with no `semantic`
- **THEN** it is schema-valid

#### Scenario: Invalid semantic value on workflow state
- **WHEN** a Workflow state entry has `semantic: "finished"`
- **THEN** it is schema-invalid

---

### Requirement: Task state entry schema
The system SHALL define a JSON Schema for a Task state entry with fields: `name` (string, required), `description` (string, required), and `semantic` (string from closed set `[complete, blocked, failed]`, optional). The `reviewing` semantic is not valid on Task states.

#### Scenario: Task state with complete semantic
- **WHEN** a Task state entry has `semantic: complete`
- **THEN** it is schema-valid

#### Scenario: Task state without semantic
- **WHEN** a Task state entry has `name` and `description` with no `semantic`
- **THEN** it is schema-valid

#### Scenario: Task state with reviewing semantic
- **WHEN** a Task state entry has `semantic: reviewing`
- **THEN** it is schema-invalid

---

### Requirement: StateMapEntry embedded type schema
The system SHALL define a JSON Schema for a `StateMapEntry` (used in Relay `state_map`) with fields: `maps_to` (string, required) and `semantic` (string from closed set `[completed, blocked, failed]`, optional).

#### Scenario: Entry with semantic
- **WHEN** a StateMapEntry has `maps_to: "Complete"` and `semantic: complete`
- **THEN** it is schema-valid

#### Scenario: Entry without semantic
- **WHEN** a StateMapEntry has only `maps_to`
- **THEN** it is schema-valid

#### Scenario: Invalid semantic on state map entry
- **WHEN** a StateMapEntry has `semantic: active`
- **THEN** it is schema-invalid

---

### Requirement: Workflow schema
The system SHALL define a JSON Schema for the `Workflow` entity with fields: `id` (CamelCase, required), `name` (string, required), `description` (string, optional), `purpose` (string, required), `accountability` (RACI, required), `steps` (array of WorkStep or ReviewStep, required, min 1), `states` (array of Workflow state entries, required, min 2), `hooks` (HooksMap, optional), `guidance` (string, optional).

#### Scenario: Valid workflow
- **WHEN** a Workflow has all required fields, at least one step, and at least two states
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

---

### Requirement: Task schema
The system SHALL define a JSON Schema for the `Task` entity with fields: `id` (CamelCase, required), `name` (string, required), `description` (string, optional), `purpose` (string, required), `instructions` (array of strings, required, min 1), `criteria` (array of strings, required, min 1), `accountability` (RACI, optional), `artifact` (object with `name` string required and `template` string optional, required), `states` (array of Task state entries, required, min 2), `hooks` (HooksMap, optional), `guidance` (string, optional).

#### Scenario: Valid task
- **WHEN** a Task has all required fields with non-empty `instructions` and `criteria`
- **THEN** it is schema-valid

#### Scenario: Missing purpose
- **WHEN** a Task omits `purpose`
- **THEN** it is schema-invalid

#### Scenario: Empty criteria
- **WHEN** a Task has `criteria` as an empty array
- **THEN** it is schema-invalid

---

### Requirement: Relay schema
The system SHALL define a JSON Schema for the `Relay` entity with fields: `id` (CamelCase, required), `name` (string, required), `description` (string, optional), `purpose` (string, required), `accountability` (RACI, optional), `delegates_to` (string, required), `briefing` (string, optional), `debriefing` (string, optional), `state_map` (map of string to StateMapEntry, required, min 1), `hooks` (HooksMap, optional), `guidance` (string, optional).

#### Scenario: Valid relay
- **WHEN** a Relay has all required fields and at least one `state_map` entry
- **THEN** it is schema-valid

#### Scenario: Missing delegates_to
- **WHEN** a Relay omits `delegates_to`
- **THEN** it is schema-invalid

#### Scenario: Missing purpose
- **WHEN** a Relay omits `purpose`
- **THEN** it is schema-invalid

---

### Requirement: Entity id format
The system SHALL enforce id format patterns per entity type: kebab-case (`^[a-z][a-z0-9-]*$`) for `Team` and `Role`; CamelCase (`^[A-Z][A-Za-z0-9]*$`) for `Workflow`, `Task`, `Relay`, and `Hook`.

#### Scenario: Valid Team id
- **WHEN** a Team has `id: "platform-team"`
- **THEN** it is schema-valid

#### Scenario: Invalid Team id (CamelCase)
- **WHEN** a Team has `id: "PlatformTeam"`
- **THEN** it is schema-invalid

#### Scenario: Valid Workflow id
- **WHEN** a Workflow has `id: "Initiative"`
- **THEN** it is schema-valid

#### Scenario: Invalid Workflow id (kebab)
- **WHEN** a Workflow has `id: "my-initiative"`
- **THEN** it is schema-invalid
