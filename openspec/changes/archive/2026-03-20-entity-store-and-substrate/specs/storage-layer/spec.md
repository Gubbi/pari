## MODIFIED Requirements

### Requirement: Substrate trait
The system SHALL define a `Substrate` trait and import `EntityStore` from `src/schema::store` in `src/substrate/mod.rs`. `EntityStore` is a neutral container of all validated entity collections; it carries no "repo" concept. The trait exposes `persist(&self, store: &EntityStore) -> Result<(), Vec<SubstrateError>>`. What varies across implementations is the persistence target, not what is stored. Future capabilities (e.g., `load`) will be added to this trait in subsequent proposals.

#### Scenario: RepoSubstrate implements Substrate
- **WHEN** `RepoSubstrate` is used as a `Substrate` implementor
- **THEN** it satisfies the trait contract and can be used wherever `Substrate` is required

---

### Requirement: RepoSubstrate accepts a caller-provided root path
The system SHALL provide `RepoSubstrate::new(root: impl Into<PathBuf>) -> Self` in `src/substrate/repo/storage.rs`. The root directory is determined entirely by the caller. No default path (e.g., `.pari/`) is hardcoded inside the substrate layer.

#### Scenario: Arbitrary root path accepted
- **WHEN** `RepoSubstrate::new` is called with any valid path
- **THEN** `persist()` writes all entity files under that path

---

### Requirement: persist is all-or-nothing
The system SHALL ensure `persist()` leaves no partial state at the target root. All entity files are written to a sibling `<dirname>.part/` temp directory first. If all writes succeed, the temp directory is atomically renamed to the target root. If any write fails, the temp directory is deleted and the original root is left untouched.

#### Scenario: Successful persist replaces target atomically
- **WHEN** all entity files are written without error
- **THEN** the temp directory is renamed to the target root atomically and no `.part/` directory remains

#### Scenario: Failed persist leaves no partial state
- **WHEN** a write error occurs for any entity file
- **THEN** the temp directory is removed and the target root is unchanged

#### Scenario: persist collects all errors before returning
- **WHEN** multiple entity files fail to write
- **THEN** all `SubstrateError` values are collected and returned together

---

### Requirement: SubstrateError identifies the failing path
Each `SubstrateError` SHALL carry a `path` (the filesystem path that failed, as a string) and a `message` (human-readable description of the failure).

#### Scenario: Write failure produces SubstrateError with path
- **WHEN** writing `roles/eng-lead.md` fails due to a permission error
- **THEN** the returned `SubstrateError` has `path: "roles/eng-lead.md"` and a descriptive message
