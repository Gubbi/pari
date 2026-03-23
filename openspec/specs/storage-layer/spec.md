## Requirements

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

### Requirement: RepoSubstrate accepts a caller-provided root path
The system SHALL provide `RepoSubstrate::new(root: impl Into<PathBuf>) -> Self` in `src/substrate/repo/storage.rs`. The root directory is determined entirely by the caller. No default path (e.g., `.pari/`) is hardcoded inside the substrate layer.

#### Scenario: Arbitrary root path accepted
- **WHEN** `RepoSubstrate::new` is called with any valid path
- **THEN** `persist()` writes all entity files under that path

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

---

### Requirement: Entity directory structure
The system SHALL write entity files under the root directory following this layout:

```
<root>/
  roles/<id>.md
  teams/<id>.md
  shared/
    hooks/<id>.md
    workflows/<WorkflowId>/
      README.md
      <TaskId>/
        README.md
        <artifact.name>.template.md  (if artifact.template is set)
      <InlineWorkflowId>/
        README.md
        <NestedTaskId>/
          README.md
  workflows/<WorkflowId>/
    README.md
    <TaskId>/
      README.md
      <artifact.name>.template.md  (if artifact.template is set)
    <RelayId>/
      README.md
    <InlineWorkflowId>/
      README.md
```

Directory names for hierarchical entities match the entity's `id`. File names for flat entities (`roles/`, `teams/`, `shared/hooks/`) match the entity's `id` with `.md` extension.

#### Scenario: Role written as flat file
- **WHEN** a Role with id `eng-lead` is persisted
- **THEN** the file `roles/eng-lead.md` is created under the root

#### Scenario: Workflow written as directory with README
- **WHEN** a Workflow with id `Initiative` is persisted
- **THEN** `workflows/Initiative/README.md` is created

#### Scenario: Embedded Task written under parent Workflow directory
- **WHEN** a Workflow `Initiative` has a WorkStep embedding a Task with id `WriteProposal`
- **THEN** `workflows/Initiative/WriteProposal/README.md` is created

#### Scenario: Embedded Relay written under parent Workflow directory
- **WHEN** a Workflow `Initiative` has a WorkStep embedding a Relay with id `LegalReview`
- **THEN** `workflows/Initiative/LegalReview/README.md` is created

#### Scenario: Inline Workflow written under parent Workflow directory
- **WHEN** a Workflow `Initiative` has a WorkStep embedding an inline Workflow with id `Kickoff`
- **THEN** `workflows/Initiative/Kickoff/README.md` is created

#### Scenario: Artifact template file written alongside Task README
- **WHEN** a Task has `artifact.template` set to non-empty content
- **THEN** a `<artifact.name>.template.md` file is written alongside the Task's `README.md`

#### Scenario: ReviewStep has no directory
- **WHEN** a Workflow has a ReviewStep
- **THEN** no directory is created for it; it is represented only in the parent Workflow's frontmatter

---

### Requirement: File format — YAML frontmatter and markdown body
Each entity file SHALL consist of a YAML frontmatter block (delimited by `---`) followed by a markdown body. The frontmatter carries structured machine-readable data. The markdown body uses standardized section headings for human-readable content. The field-to-location mapping is:

| Field | Frontmatter | Markdown section |
|---|---|---|
| `id` | yes | — |
| `name` | — | `# <name>` (H1 title) |
| `description` | — | paragraph after H1 |
| `purpose` | — | `## Purpose` |
| `accountability` | yes | — |
| `steps` | yes | — |
| `states` | yes | — |
| `state_map` | yes | — |
| `delegates_to` | yes | — |
| `artifact` | yes | — |
| `hooks` | yes | — |
| `members` | yes | — |
| `include` | yes | — |
| `traits` | — | `## Responsibilities` |
| `instructions` (Hook) | — | `## Instructions` |
| `instructions` (Task) | — | `## Steps` |
| `criteria` | — | `## Criteria` |
| `briefing` | — | `## Briefing` |
| `debriefing` | — | `## Debriefing` |
| `guidance` | — | `## Guidance` |
| `extensions` (`x-` keys) | yes | — |

#### Scenario: Frontmatter is valid YAML
- **WHEN** an entity file is written
- **THEN** the frontmatter block parses as valid YAML

#### Scenario: Optional fields omitted from frontmatter when absent
- **WHEN** an entity has no `hooks`
- **THEN** the `hooks` key is absent from the frontmatter (not written as null)

#### Scenario: Extensions written to frontmatter
- **WHEN** an entity has extensions `{ "x-owner": "platform" }`
- **THEN** `x-owner: platform` appears in the frontmatter

#### Scenario: Markdown body sections only written when field is present
- **WHEN** an entity has no `guidance`
- **THEN** no `## Guidance` section appears in the markdown body

---

### Requirement: SubstrateError identifies the failing path
Each `SubstrateError` SHALL carry a `path` (the filesystem path that failed, as a string) and a `message` (human-readable description of the failure).

#### Scenario: Write failure produces SubstrateError with path
- **WHEN** writing `roles/eng-lead.md` fails due to a permission error
- **THEN** the returned `SubstrateError` has `path: "roles/eng-lead.md"` and a descriptive message
