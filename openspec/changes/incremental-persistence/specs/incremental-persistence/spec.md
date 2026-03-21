## ADDED Requirements

### Requirement: ChangeSet represents a substrate-agnostic set of entity changes
The system SHALL define a `ChangeSet` type in `src/substrate/changeset.rs` containing a flat `Vec<EntityChange>`. Each `EntityChange` SHALL carry:
- `path: String` — tree location (e.g., `"workflows/Initiative/WriteProposal"`)
- `kind: EntityKind` — the entity kind (Role, Hook, Team, Workflow, Task, Relay, SharedWorkflow)
- `id: String` — the entity's id
- `op: ChangeOp` — one of `Added(EntityData)`, `Modified { entity: EntityData, dirty_fields: Vec<String> }`, or `Removed`

`EntityData` SHALL be an enum carrying the full plain entity value for each entity type.

#### Scenario: Added entity in changeset
- **WHEN** a new Role `"eng-lead"` is added to the store and changes are drained
- **THEN** the ChangeSet contains an `EntityChange` with `path: "roles"`, `kind: Role`, `id: "eng-lead"`, `op: Added(role_data)`

#### Scenario: Modified entity carries dirty field names and full entity
- **WHEN** the `name` field of Role `"eng-lead"` is changed and changes are drained
- **THEN** the ChangeSet contains an `EntityChange` with `op: Modified { entity: full_role, dirty_fields: ["name"] }`

#### Scenario: Removed entity in changeset
- **WHEN** Role `"eng-lead"` is removed from the store and changes are drained
- **THEN** the ChangeSet contains an `EntityChange` with `path: "roles"`, `kind: Role`, `id: "eng-lead"`, `op: Removed`

#### Scenario: Nested workflow step changes produce flat entries
- **WHEN** a Task `"WriteProposal"` nested inside Workflow `"Initiative"` has its `instructions` field modified
- **THEN** the ChangeSet contains an `EntityChange` with `path: "workflows/Initiative/WriteProposal"`, `kind: Task`, `id: "WriteProposal"`, `op: Modified { dirty_fields: ["instructions"] }`

---

### Requirement: EntityStore::drain_changes produces a ChangeSet
The system SHALL provide `EntityStore::drain_changes(&mut self) -> ChangeSet` that walks all tracked entity collections, collects changes into a flat `ChangeSet`, and resets all dirty flags. After `drain_changes()`, a subsequent call with no intervening mutations SHALL return an empty `ChangeSet`.

#### Scenario: drain_changes collects all entity types
- **WHEN** changes exist across roles, hooks, and workflows
- **THEN** `drain_changes()` returns a ChangeSet containing entries for all three

#### Scenario: drain_changes resets dirty state
- **WHEN** `drain_changes()` is called
- **THEN** a subsequent `drain_changes()` with no mutations returns an empty ChangeSet

#### Scenario: drain_changes walks nested workflow steps
- **WHEN** a Workflow has a dirty nested Task three levels deep
- **THEN** `drain_changes()` produces a flat EntityChange with the full path to that Task

---

### Requirement: RepoSubstrate uses LCA-based atomic persistence
The system SHALL compute the lowest common ancestor (LCA) directory of all changed file paths in the ChangeSet. Persistence SHALL stage changes within only the LCA subtree:
1. Create a `.part/` sibling directory of the LCA directory
2. Hard-link unchanged files within the LCA subtree into `.part/`
3. Write changed files (re-rendered) into `.part/`
4. Omit removed files from `.part/`
5. Rename the LCA directory to `.old/`, rename `.part/` to the LCA directory, delete `.old/`

If hard-linking fails with a cross-device error, the system SHALL fall back to file copy.

#### Scenario: Single entity change swaps only parent directory
- **WHEN** only `roles/eng-lead.md` changed and `roles/` contains 3 files total
- **THEN** only the `roles/` directory is swapped (2 hard-links + 1 write + rename)

#### Scenario: Changes in same subtree swap the common ancestor
- **WHEN** `workflows/Initiative/README.md` and `workflows/Initiative/WriteProposal/README.md` both changed
- **THEN** `workflows/Initiative/` is the LCA and only that subtree is swapped

#### Scenario: Changes across top-level directories swap root
- **WHEN** both `roles/eng-lead.md` and `workflows/Initiative/README.md` changed
- **THEN** root is the LCA and the entire root directory is swapped (equivalent to full snapshot)

#### Scenario: Initial persist with no existing root
- **WHEN** `persist()` is called and no root directory exists
- **THEN** all files are written to `.part/` and renamed to root (identical to current full-snapshot behavior)

#### Scenario: Empty changeset is a no-op
- **WHEN** `persist()` is called with an empty ChangeSet
- **THEN** no filesystem operations are performed

#### Scenario: Cross-device hard-link falls back to copy
- **WHEN** hard-linking a file fails with `EXDEV` (cross-device)
- **THEN** the file is copied instead and persistence continues normally
