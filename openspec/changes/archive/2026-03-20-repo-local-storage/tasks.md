## 1. Dependencies and module scaffolding

- [x] 1.1 Promote `serde_json` to regular dependency and add `serde_yaml` in Cargo.toml
- [x] 1.2 Create `src/storage/mod.rs` with stub `Storage` trait, `StorageError`, and `RepoStore`; expose `pub mod storage` in `src/lib.rs`

## 2. Extensions type

- [x] 2.1 Write tests for `Extensions`: serde round-trip with `x-` keys; schemars emits `patternProperties: { "^x-": {} }` when flattened into a struct (note: `additionalProperties: false` is separate — from `#[schemars(deny_unknown_fields)]` on entity structs, tested in step 10)
- [x] 2.2 Implement `Extensions` newtype in `types.rs`; add `#[serde(flatten)]` field to a test struct; if schemars does not emit `patternProperties` natively, add xtask post-processing and document the path taken

## 3. Extensions validation helper

- [x] 3.1 Write tests for `validate_extensions`: all `x-` keys pass; non-prefixed key produces `ValidationError` at correct path; empty extensions passes
- [x] 3.2 Implement `validate_extensions(extensions: &Extensions, path: &str) -> Vec<ValidationError>` in `validation.rs`

## 4. WorkStepDefinition and restructured step types

- [x] 4.1 Write tests for `WorkStepDefinition` untagged serde discrimination: `artifact` → Task variant; `delegates_to` → Relay variant; `steps` → Workflow variant; `id()` returns inner entity id
- [x] 4.2 Implement `WorkStepDefinition` untagged enum and restructured `WorkStep { depends_on, definition }` in `types.rs`; update `Step::id()` to delegate to `definition.id()`
- [x] 4.3 Write tests for `SharedWorkStepDefinition` (Task and SharedWorkflow variants only); `SharedWorkStep`; `SharedStep` (SharedWorkStep | ReviewStep)
- [x] 4.4 Implement `SharedWorkStepDefinition`, `SharedWorkStep`, and `SharedStep` in `types.rs`

## 5. WorkflowDef generic and SharedWorkflow entity

- [x] 5.1 Write tests for `WorkflowDef<Step>` (Workflow) and `WorkflowDef<SharedStep>` (SharedWorkflow) construction with all required fields
- [x] 5.2 Implement `WorkflowDef<S>` generic struct in `entities/workflow.rs`; make `Workflow = WorkflowDef<Step>` and `SharedWorkflow = WorkflowDef<SharedStep>`
- [x] 5.3 Update existing `workflow.rs` tests to use the new `WorkStep { depends_on, definition }` shape; verify all tests compile and pass

## 6. RepoContext typed workflow collections

- [x] 6.1 Write tests for `RepoContext` with typed `workflows: Vec<Workflow>` and `shared_workflows: Vec<SharedWorkflow>`; `has_shared_workflow` and `get_shared_workflow_states` use the typed vec
- [x] 6.2 Update `context.rs`: replace `shared_workflows: HashMap<String, SharedWorkflowInfo>` with `shared_workflows: Vec<SharedWorkflow>`; update `has_shared_workflow` and `get_shared_workflow_states` accordingly; update existing context tests

## 7. Validator updates — depends_on and composition

- [x] 7.1 Write tests for `validate_work_step_depends_on` using embedded entity ids (not a standalone `id` field on WorkStep)
- [x] 7.2 Update `validate_work_step_depends_on` in `workflow.rs` to derive step names from `WorkStepDefinition.id()`
- [x] 7.3 Write tests for validator composition: embedded Task structural error appears at `steps[i].definition.<field>`; embedded Relay semantic error appears at `steps[i].definition.<field>`; inline Workflow errors recurse with full path; structural errors present → semantic validation skipped
- [x] 7.4 Implement validator composition in `workflow::validate`: split into `validate_structure_tree` and `validate_semantic_tree`; call `task::validate`, `relay::validate`, and `workflow::validate` recursively for embedded definitions; prefix all child errors with `steps[i].definition`

## 8. Validator updates — Extensions on all entities

- [x] 8.1 Add `extensions: Extensions` field to `Role`, `Hook`, `Team`, `WorkflowDef`, `Task`, `Relay` structs; update all constructor calls in existing tests to include the field
- [x] 8.2 Write tests for `validate_extensions` called from each entity validator: Role, Hook, Team, Workflow, Task, Relay each reject non-`x-` extension keys
- [x] 8.3 Add `validate_extensions(&entity.extensions, "extensions")` call (Phase 1) to each entity's validator: `role::validate`, `hook::validate`, `team::validate`, `workflow::validate`, `task::validate`, `relay::validate`

## 9. Validator updates — new rules

- [x] 9.1 Write tests for SharedWorkflow structural validation: Relay step definition rejected; Task and ReviewStep definitions accepted
- [x] 9.2 Implement `shared_workflow::validate` in `workflow.rs`: validates SharedWorkflow including the no-Relay-step constraint; calls child validators for embedded entities
- [x] 9.3 Write tests for updated `relay::validate` against typed `RepoContext.shared_workflows` (vec lookup instead of string set)
- [x] 9.4 Update `relay.rs` `delegates_to` validation to look up against `ctx.shared_workflows` by id field

## 10. Schema generation updates

- [x] 10.1 Update xtask: remove `write_schema::<Task>` and `write_schema::<Relay>` calls; delete `schemas/task.json` and `schemas/relay.json`; add `write_schema::<SharedWorkflow>` and any new shared types
- [x] 10.2 Add `#[schemars(deny_unknown_fields)]` to all entity structs; run xtask; verify entity schemas include both `patternProperties: { "^x-": {} }` (from Extensions) and `additionalProperties: false` (from deny_unknown_fields)

## 11. Storage trait and core types

- [x] 11.1 Write tests for `Storage` trait contract: `StorageError` has `path` and `message`; `EntityStore` holds all entity collections
- [x] 11.2 Implement `Storage` trait, `StorageError`, and `EntityStore` in `src/storage/mod.rs`; expose `pub mod repo` from `mod.rs`; remove `RepoStore` (replaced by `EntityStore`)

## 12. RepoStorage — atomic persist

- [x] 12.1 Write tests for `RepoStorage::new` with arbitrary paths; `persist()` creates `<dirname>.part/` temp directory; on success renames to target atomically; on write error removes temp directory and returns errors
- [x] 12.2 Implement `RepoStorage` in `src/storage/repo/storage.rs` with `persist()` atomic all-or-nothing logic; implement `Storage` for `RepoStorage`

## 13. Entity render functions

- [x] 13.1 Write tests for `render_role` in `src/storage/repo/render.rs`: produces valid YAML frontmatter with `id` and `x-` keys; markdown body has `# <name>`, `## Purpose`, `## Responsibilities` (when traits present); optional sections omitted when fields absent
- [x] 13.2 Implement `render_role(role: &Role) -> String` in `src/storage/repo/render.rs`
- [x] 13.3 Write tests for `render_hook`: frontmatter with `id`; markdown body with `# <name>`, `## Instructions`; optional `## Purpose` and `## Guidance`
- [x] 13.4 Implement `render_hook(hook: &Hook) -> String`
- [x] 13.5 Write tests for `render_team`: frontmatter with `id`, `members`, `include`; markdown body with `# <name>`; optional sections omitted when absent
- [x] 13.6 Implement `render_team(team: &Team) -> String`
- [x] 13.7 Write tests for `render_workflow_readme`: frontmatter with `id`, `accountability`, `steps`, `states`, `hooks`; markdown body sections; ReviewStep represented in `steps` frontmatter only, no directory created
- [x] 13.8 Implement `render_workflow_readme(workflow: &WorkflowDef<impl Serialize>) -> String`
- [x] 13.9 Write tests for `render_task_readme`: frontmatter with `id`, `artifact`, `states`, `hooks`; markdown body with `## Steps`, `## Criteria`, `## Guidance`; template file content when `artifact.template` is set
- [x] 13.10 Implement `render_task_readme(task: &Task) -> String`; write `<artifact.name>.template.md` alongside `README.md` when `artifact.template` is set
- [x] 13.11 Write tests for `render_relay_readme`: frontmatter with `id`, `delegates_to`, `state_map`; markdown body with `## Briefing`, `## Debriefing` when present
- [x] 13.12 Implement `render_relay_readme(relay: &Relay) -> String`

## 14. Integration

- [x] 14.1 Write integration tests for `persist()` with a minimal `EntityStore` covering all entity types: verify full directory tree structure matches spec; verify frontmatter parses as valid YAML; verify template files created when `artifact.template` is set
- [x] 14.2 Run full test suite; confirm all tests pass (`cargo test`)
