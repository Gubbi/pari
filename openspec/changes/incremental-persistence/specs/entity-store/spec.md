## MODIFIED Requirements

### Requirement: EntityStore is a HashMap-keyed collection of validated entities
The system SHALL define `EntityStore` in `src/schema/store.rs` as the single canonical collection of all validated entities. Internally, EntityStore SHALL use `TrackedMap<String, TrackedEntity>` for each entity collection to enable change tracking. The public API SHALL accept plain entity types at insertion boundaries but return tracked instances (e.g., `&TrackedRole`) which provide plain field references via `Deref` on reads.

EntityStore SHALL provide typed insertion methods:
- `insert_role(role: Role)`
- `insert_hook(hook: Hook)`
- `insert_team(team: Team)`
- `insert_shared_workflow(wf: SharedWorkflow)`
- `insert_workflow(wf: Workflow)`

Each insertion method SHALL convert the plain entity to its tracked variant and insert into the corresponding TrackedMap.

`EntityStore` serves dual purpose: validation context (passed to all `validate()` calls) and persistence input (produces `ChangeSet` via `collect_changes()`). The incoming entity being validated SHALL NOT be present in the store — callers are responsible for this invariant.

#### Scenario: EntityStore holds entities by id key
- **WHEN** a `Role` with id `eng-lead` is inserted via `store.insert_role(role)`
- **THEN** `store.has_role("eng-lead")` returns true and `store.get_role("eng-lead")` returns a reference to the role's fields

#### Scenario: EntityStore is empty by default
- **WHEN** `EntityStore::new()` is called
- **THEN** all five collections are empty

#### Scenario: Insertion converts plain to tracked internally
- **WHEN** a plain `Role` is inserted via `insert_role()`
- **THEN** the role is stored as a `TrackedRole` internally, and the insertion is recorded in the TrackedMap's dirty set

#### Scenario: Read access returns tracked instance with transparent field access
- **WHEN** `store.get_role("eng-lead")` is called
- **THEN** the returned `&TrackedRole` provides access to plain `String`, `Option<Vec<String>>`, etc. fields via Deref on each tracked field

---

### Requirement: EntityStore exposes lookup methods for validation
The system SHALL provide the following methods on `EntityStore` for use by entity validators:

- `has_role(id: &str) -> bool`
- `has_hook(id: &str) -> bool`
- `has_team(id: &str) -> bool`
- `has_shared_workflow(id: &str) -> bool`
- `get_hook(id: &str) -> Option<&TrackedHook>` — fields accessible via Deref
- `get_team(id: &str) -> Option<&TrackedTeam>` — fields accessible via Deref
- `get_shared_workflow_states(id: &str) -> Option<Vec<String>>`

#### Scenario: has_role returns true for a known role
- **WHEN** a role with id `eng-lead` is in `EntityStore`
- **THEN** `store.has_role("eng-lead")` returns true

#### Scenario: has_role returns false for an unknown role
- **WHEN** no role with id `unknown` is in `EntityStore`
- **THEN** `store.has_role("unknown")` returns false

#### Scenario: get_hook returns the full Hook entity
- **WHEN** a hook with id `UpdateJira` is in `EntityStore`
- **THEN** `store.get_hook("UpdateJira")` returns `Some(&TrackedHook)` with fields accessible via Deref, including inputs

#### Scenario: get_shared_workflow_states returns state ids in order
- **WHEN** a shared workflow with id `LegalReview` has states `["Active", "Done"]`
- **THEN** `store.get_shared_workflow_states("LegalReview")` returns `Some(vec!["Active", "Done"])`

#### Scenario: get_shared_workflow_states returns None for unknown id
- **WHEN** no shared workflow with id `Unknown` is in `EntityStore`
- **THEN** `store.get_shared_workflow_states("Unknown")` returns `None`

---

### Requirement: EntityStore exposes mutable access for entity modification
The system SHALL provide mutable access methods that return tracked references, enabling transparent dirty-marking via DerefMut:

- `get_role_mut(id: &str) -> Option<&mut TrackedRole>`
- `get_hook_mut(id: &str) -> Option<&mut TrackedHook>`
- `get_team_mut(id: &str) -> Option<&mut TrackedTeam>`
- `get_shared_workflow_mut(id: &str) -> Option<&mut TrackedSharedWorkflow>`
- `get_workflow_mut(id: &str) -> Option<&mut TrackedWorkflow>`
- `remove_role(id: &str)`
- `remove_hook(id: &str)`
- `remove_team(id: &str)`
- `remove_shared_workflow(id: &str)`
- `remove_workflow(id: &str)`

#### Scenario: Mutable access auto-tracks field changes
- **WHEN** `store.get_role_mut("eng-lead")` is used to modify the `name` field
- **THEN** the `name` field is marked dirty and appears in the next `collect_changes()` result

#### Scenario: Remove records deletion in tracked map
- **WHEN** `store.remove_role("eng-lead")` is called
- **THEN** the role is removed and `"eng-lead"` appears in the removed set of the next `collect_changes()`
