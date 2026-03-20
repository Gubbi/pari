## Requirements

### Requirement: EntityStore is a HashMap-keyed collection of validated entities
The system SHALL define `EntityStore` in `src/schema/store.rs` as the single canonical collection of all validated entities, keyed by entity id for O(1) lookup. Fields:

```
pub struct EntityStore {
    pub roles:            HashMap<String, Role>,
    pub hooks:            HashMap<String, Hook>,
    pub teams:            HashMap<String, Team>,
    pub shared_workflows: HashMap<String, SharedWorkflow>,
    pub workflows:        HashMap<String, Workflow>,
}
```

`EntityStore` serves dual purpose: validation context (passed to all `validate()` calls) and persistence input (passed to `persist()`). The incoming entity being validated SHALL NOT be present in the store — callers are responsible for this invariant.

#### Scenario: EntityStore holds entities by id key
- **WHEN** a `Role` with id `eng-lead` is inserted into `EntityStore`
- **THEN** `store.roles.contains_key("eng-lead")` returns true and `store.roles.get("eng-lead")` returns the full `Role`

#### Scenario: EntityStore is empty by default
- **WHEN** `EntityStore::new()` is called
- **THEN** all five collections are empty

---

### Requirement: EntityStore exposes lookup methods for validation
The system SHALL provide the following methods on `EntityStore` for use by entity validators:

- `has_role(id: &str) -> bool`
- `has_hook(id: &str) -> bool`
- `has_team(id: &str) -> bool`
- `has_shared_workflow(id: &str) -> bool`
- `get_hook(id: &str) -> Option<&Hook>`
- `get_team(id: &str) -> Option<&Team>`
- `get_shared_workflow_states(id: &str) -> Option<Vec<String>>`

#### Scenario: has_role returns true for a known role
- **WHEN** a role with id `eng-lead` is in `EntityStore`
- **THEN** `store.has_role("eng-lead")` returns true

#### Scenario: has_role returns false for an unknown role
- **WHEN** no role with id `unknown` is in `EntityStore`
- **THEN** `store.has_role("unknown")` returns false

#### Scenario: get_hook returns the full Hook entity
- **WHEN** a hook with id `UpdateJira` is in `EntityStore`
- **THEN** `store.get_hook("UpdateJira")` returns `Some(&Hook)` with the full entity including inputs

#### Scenario: get_shared_workflow_states returns state ids in order
- **WHEN** a shared workflow with id `LegalReview` has states `["Active", "Done"]`
- **THEN** `store.get_shared_workflow_states("LegalReview")` returns `Some(vec!["Active", "Done"])`

#### Scenario: get_shared_workflow_states returns None for unknown id
- **WHEN** no shared workflow with id `Unknown` is in `EntityStore`
- **THEN** `store.get_shared_workflow_states("Unknown")` returns `None`

---

### Requirement: Team exposes a get_refs method for cycle detection
The system SHALL provide `Team::get_refs() -> impl Iterator<Item = &str>` that yields all team ids directly referenced by the team via both `include` (map keys) and `import` (list entries). This is the authoritative source for BFS-based cycle detection.

#### Scenario: get_refs yields include keys and import entries
- **WHEN** a team has `include: { "backend-team": "eng" }` and `import: ["design-team"]`
- **THEN** `team.get_refs()` yields `"backend-team"` and `"design-team"`

#### Scenario: get_refs yields nothing for a team with no references
- **WHEN** a team has neither `include` nor `import`
- **THEN** `team.get_refs()` yields no items
