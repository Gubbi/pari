## Why

Pari's entity definitions — workflows, tasks, roles, teams, hooks — need to live inside the user's repository as version-controlled, human-readable files. The first persistence target is the local filesystem: after schema validation, entities are serialized to a structured directory tree within a `.pari/` root. This establishes the authoritative source of truth for the Definition Layer and makes it accessible to every participant without any external tooling.

## What Changes

- Introduce a `repo-local-storage` persistence target that serializes validated entity instances to markdown files with YAML frontmatter
- Add `Extensions` newtype (`HashMap<String, serde_json::Value>`) with a custom `JsonSchema` impl; apply `#[schemars(deny_unknown_fields)]` on all entity structs to emit `patternProperties: { "^x-": true }` + `additionalProperties: false`
- **BREAKING**: Refactor `Step` enum — `WorkStep` changes from `{ id, depends_on }` to a wrapper `{ depends_on, definition: WorkStepDefinition }`; `WorkStepDefinition` is `Task | Relay | Box<Workflow>`; bare named steps (WorkStep with only an id) are no longer valid — every work step must embed a definition
- **BREAKING**: Introduce `WorkflowDef<S>` generic struct; `Workflow` and `SharedWorkflow` become type aliases with different step enums (`Step` vs `SharedStep`); `SharedStep` excludes `Relay`
- `RepoContext` gains typed collections: `workflows: Vec<Workflow>`, `shared_workflows: Vec<SharedWorkflow>`
- `Task` and `Relay` are no longer standalone top-level entities — they exist only as embedded `WorkStepDefinition` variants within a parent workflow

## Directory Structure

```
<root>/                          # default: .pari/ — configurable via pari.yaml
  roles/
    <id>.md
  teams/
    <id>.md
  workflows/
    <WorkflowId>/
      README.md
      <TaskId>/
        README.md
        <artifact-name>.template.md
      <RelayId>/
        README.md
      <InlineWorkflowId>/
        README.md
        <NestedTaskId>/
          README.md
  shared/
    hooks/
      <id>.md
    workflows/
      <WorkflowId>/
        README.md
        <TaskId>/
          README.md
```

**Naming conventions:**
- Flat entities (Role, Team, Hook): `<id>.md` directly under their type directory
- Hierarchical entities (Workflow, Task, Relay, inline Workflow): `README.md` inside a directory named `<id>`
- Template files: `<artifact.name>.template.md` alongside the owning Task's `README.md`
- Entity `id` is authoritative in frontmatter; the directory or file name must match it
- `ReviewStep` has no file — it is represented inline within the parent workflow's `steps` frontmatter only; no subdirectory is created for it
- Subdirectory names (`roles/`, `teams/`, `workflows/`, `shared/`) are fixed — not configurable
- The root directory defaults to `.pari/` and is the only configurable path (via `pari.yaml`)

**Entity type discrimination** is by directory position:
- `.pari/workflows/<Id>/README.md` → `Workflow`
- `.pari/workflows/<Id>/<StepId>/README.md` → `Task`, `Relay`, or inline `Workflow` (determined by frontmatter)
- `.pari/shared/workflows/<Id>/README.md` → `SharedWorkflow`

## File Format

Each file is markdown with a YAML frontmatter block. The frontmatter carries machine-readable structured data. The markdown body carries human-readable content using standardized section headings.

**User extensions:**
- Frontmatter: any key prefixed `x-` (e.g. `x-hiring: true`). Non-`x-` unknown keys are a validation error.
- Markdown: any additional section using `## [Tag] Title` pattern (e.g. `## [Onboarding] Getting Started`). Standard section names (no brackets) are reserved for Pari.

### Frontmatter and Markdown Section Mapping

| Field | Role | Team | Hook | Workflow | Task | Relay |
|---|---|---|---|---|---|---|
| `id` | FM | FM | FM | FM | FM | FM |
| `name` | `# Title` | `# Title` | `# Title` | `# Title` | `# Title` | `# Title` |
| `description` | body after H1 | body after H1 | body after H1 | body after H1 | body after H1 | body after H1 |
| `purpose` | `## Purpose` | `## Purpose` | `## Purpose` | `## Purpose` | `## Purpose` | `## Purpose` |
| `instructions` (Hook) | — | — | `## Instructions` | — | — | — |
| `instructions` (Task) | — | — | — | — | `## Steps` | — |
| `criteria` | — | — | — | — | `## Criteria` | — |
| `briefing` | — | — | — | — | — | `## Briefing` |
| `debriefing` | — | — | — | — | — | `## Debriefing` |
| `guidance` | — | — | — | `## Guidance` | `## Guidance` | `## Guidance` |
| `traits` | `## Responsibilities` | — | — | — | — | — |
| `accountability` | — | — | — | FM | FM (opt) | FM (opt) |
| `steps` | — | — | — | FM | — | — |
| `depends_on` | — | — | — | FM (in `steps`) | — | — |
| `states` | — | — | — | FM | FM | — |
| `state_map` | — | — | — | — | — | FM |
| `artifact` | — | — | — | — | FM | — |
| `delegates_to` | — | — | — | — | — | FM |
| `hooks` | — | — | — | FM | FM | FM |
| `members` | — | FM | — | — | — | — |
| `include` | — | FM | — | — | — | — |

## Capabilities

### New Capabilities

- `repo-local-storage`: Filesystem serializer that walks validated entity instances and writes the `.pari/` directory tree per the structure and conventions above

### Modified Capabilities

- `entity-schemas`: `WorkflowDef<S>` generic, `Step`/`SharedStep` enum restructuring, `WorkStepDefinition`/`SharedWorkStepDefinition`, `Extensions` type on all entities
- `schema-validation`: Four new/updated rules — (1) shared workflows must not contain `Relay` steps; (2) extension keys must match `^x-`; (3) `depends_on` references must resolve against step ids derived from embedded entity ids (since step id is now `WorkStepDefinition`'s inner entity id, not a standalone field); (4) `Relay.delegates_to` must reference the id of a `SharedWorkflow` in `RepoContext.shared_workflows` (existing rule updated — context now holds `Vec<SharedWorkflow>` instead of `Vec<String>`)

## Impact

- `src/schema/types.rs` — `WorkflowDef<S>`, `Step`, `SharedStep`, `WorkStep`, `WorkStepDefinition`, `SharedWorkStep`, `SharedWorkStepDefinition`, `Extensions`
- `src/schema/entities/workflow.rs` — updated to `WorkflowDef<Step>`; `SharedWorkflow` as `WorkflowDef<SharedStep>`
- `src/schema/entities/task.rs`, `relay.rs` — embedded only, no longer top-level
- `src/schema/context.rs` — `workflows: Vec<Workflow>`, `shared_workflows: Vec<SharedWorkflow>`
- `src/schema/validation.rs` — four new/updated validation rules above; `depends_on` validation updated for embedded step ids
- `src/storage/repo_local.rs` — new serializer module
- `schemas/` — regenerated with `patternProperties` + `additionalProperties: false`
- New dependency: `serde_yaml`
