## MODIFIED Requirements

### Requirement: Substrate trait
The system SHALL define a `Substrate` trait in `src/substrate/mod.rs`. The trait exposes `atomic_persist(&self, changeset: &ChangeSet) -> Result<(), Vec<SubstrateError>>`. The rename makes the atomicity guarantee explicit. The substrate receives a pre-built `ChangeSet` and persists only the changes described in it. The substrate does not interact with `EntityStore` directly — change detection is the caller's responsibility via `EntityStore::collect_changes()`.

The expected call sequence is:
```
let cs = store.collect_changes();
substrate.atomic_persist(&cs)?;
store.reset_tracked();
```

#### Scenario: RepoSubstrate implements Substrate
- **WHEN** `RepoSubstrate` is used as a `Substrate` implementor
- **THEN** it satisfies the trait contract and can be used wherever `Substrate` is required

#### Scenario: atomic_persist accepts a ChangeSet
- **WHEN** `substrate.atomic_persist(&changeset)` is called with a ChangeSet containing one modified role
- **THEN** only the modified role's file is re-rendered and written

---

### Requirement: atomic_persist is all-or-nothing
The system SHALL ensure `atomic_persist()` leaves no partial state at the target root. For the LCA-based approach, all changes within the LCA subtree are staged in a `.part/` directory. If all writes succeed, the LCA directory is atomically swapped. If any write fails, the `.part/` directory is deleted and the existing state is unchanged.

#### Scenario: Successful persist swaps LCA directory atomically
- **WHEN** all changed entity files are written without error
- **THEN** the LCA subtree is atomically swapped and no `.part/` directory remains

#### Scenario: Failed persist leaves no partial state
- **WHEN** a write error occurs for any entity file
- **THEN** the `.part/` directory is removed and the existing state is unchanged

#### Scenario: atomic_persist collects all errors before returning
- **WHEN** multiple entity files fail to write
- **THEN** all `SubstrateError` values are collected and returned together
