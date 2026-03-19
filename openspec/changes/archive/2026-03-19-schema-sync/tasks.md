## 1. Dependencies and xtask scaffold

- [x] 1.1 Add `serde` (with derive feature) and `schemars` to `[dependencies]` in `Cargo.toml`
- [x] 1.2 Create `xtask/` crate with its own `Cargo.toml` and stub `src/main.rs`
- [x] 1.3 Add `xtask` to `[workspace]` in root `Cargo.toml` and add `pari` as a dependency of `xtask`

## 2. Shared types (types.rs)

- [x] 2.1 Write schema coherence tests for shared types: `Raci` (required fields), `HookInvocation` (`oneOf` string/object shape), `Step` (`oneOf` WorkStep/ReviewStep shape), `WorkflowStateEntry`/`TaskStateEntry`/`StateMapEntry` (semantic enum values)
- [x] 2.2 Add `#[derive(Serialize, Deserialize, JsonSchema)]` to all types in `types.rs`; add `#[serde(untagged)]` to `HookInvocation` and `Step`; add `#[serde(rename_all = "snake_case")]` to semantic enums

## 3. Role

- [x] 3.1 Write schema coherence test: `role.id` generated schema contains `pattern: ^[a-z][a-z0-9-]*$`
- [x] 3.2 Add derives and `#[schemars(regex(pattern = r"^[a-z][a-z0-9-]*$"))]` on `id` in `Role`

## 4. Hook

- [x] 4.1 Write schema coherence tests: `hook.id` has CamelCase pattern; `hook.instructions` has `minItems: 1`
- [x] 4.2 Add derives and annotations to `Hook` and `HookInput`

## 5. Team

- [x] 5.1 Write schema coherence test: `team.id` generated schema contains kebab-case pattern
- [x] 5.2 Add derives and `#[schemars(regex(...))]` on `id` in `Team`

## 6. Workflow

- [x] 6.1 Write schema coherence tests: `workflow.id` has CamelCase pattern; `workflow.steps` has `minItems: 1`; `workflow.states` has `minItems: 2`
- [x] 6.2 Add derives and annotations to `Workflow`

## 7. Task

- [x] 7.1 Write schema coherence tests: `task.id` has CamelCase pattern; `task.instructions` has `minItems: 1`; `task.criteria` has `minItems: 1`; `task.states` has `minItems: 2`
- [x] 7.2 Add derives and annotations to `Task`

## 8. Relay

- [x] 8.1 Write schema coherence test: `relay.id` generated schema contains CamelCase pattern
- [x] 8.2 Add derives and `#[schemars(regex(...))]` on `id` in `Relay`

## 9. xtask generate-schemas

- [x] 9.1 Implement `generate-schemas` in xtask: call `schemars::schema_for!` for each entity and shared type, write JSON to `schemas/<name>.json`
- [x] 9.2 Run `cargo xtask generate-schemas`, verify output matches expected schema shape for Role, Hook, Workflow (spot-check); commit generated schemas

## 10. Cleanup and CI

- [x] 10.1 Delete `tests/json_schema_validation.rs`
- [x] 10.2 Add CI step that runs `cargo xtask generate-schemas` and fails if output differs from committed `schemas/*.json`
