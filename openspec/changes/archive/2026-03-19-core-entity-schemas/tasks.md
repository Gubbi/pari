## 1. Project Setup

- [x] 1.1 Initialize Rust library crate with `Cargo.toml` (no external deps)
- [x] 1.2 Create `src/schema/` module layout: `types`, `entities`, `validation`, `context`; implement `ValidationError { path: String, message: String }`

## 2. JSON Schemas

- [x] 2.1 Write JSON schemas for all embedded types: `raci`, `hook_invocation`, `hooks_map`, `work_step`, `review_step`, `workflow_state_entry`, `task_state_entry`, `state_map_entry`, `artifact`
- [x] 2.2 Write JSON schemas for all six entities: `role`, `hook`, `team`, `workflow`, `task`, `relay`

## 3. Id Format Helpers

- [x] 3.1 Write tests for `is_kebab_case` and `is_camel_case` covering valid and invalid inputs
- [x] 3.2 Implement `is_kebab_case` and `is_camel_case` helpers

## 4. Role

- [x] 4.1 Write tests for Role struct shape and structural validator (id format, required fields, optional traits)
- [x] 4.2 Implement `Role` struct and its structural validator

## 5. Hook

- [x] 5.1 Write tests for Hook struct and structural validator (id format, instructions min 1, input object shape)
- [x] 5.2 Implement `HookInput`, `Hook` structs and structural validator

## 6. RACI and HooksMap Embedded Types

- [x] 6.1 Write tests for `Raci` struct (required fields, empty lists allowed)
- [x] 6.2 Implement `Raci` struct
- [x] 6.3 Write tests for `HookInvocation` (bare string, invocation object, missing hook field), `HooksMap` (single and list values)
- [x] 6.4 Implement `HookInvocation` enum and `HooksMap` type

## 7. Step Embedded Types

- [x] 7.1 Write tests for `WorkStep` (name required, optional depends_on) and `ReviewStep` (all fields required)
- [x] 7.2 Implement `WorkStep`, `ReviewStep`, and `Step` enum

## 8. State Entry Embedded Types

- [x] 8.1 Write tests for `WorkflowStateEntry` (semantic closed set includes reviewing), `TaskStateEntry` (semantic closed set excludes reviewing), and `StateMapEntry` (maps_to required, semantic from relay closed set)
- [x] 8.2 Implement `WorkflowStateEntry` with `WorkflowSemantic`, `TaskStateEntry` with `TaskSemantic`, `StateMapEntry` with `RelayStateSemantic`, and `Artifact` struct

## 9. RepoContext

- [x] 9.1 Write tests for `RepoContext` construction (manually populate known ids; assert lookups work correctly)
- [x] 9.2 Implement `RepoContext` stub in `src/schema/context.rs` — holds known role_ids, hook_ids, team_ids, and shared workflow state names

## 10. Team

- [x] 10.1 Write tests for `Team` structural validator: id format, handle regex (including dot), uniqueness of handles, presence of include/import fields
- [x] 10.2 Implement `TeamMember`, `Team` structs and structural validator
- [x] 10.3 Write tests for Team cross-entity validators: include/import referential integrity (team_ids and role_ids), member role referential integrity, conflict precedence (members > import > include, last import wins)
- [x] 10.4 Implement Team cross-entity validators (referential integrity + conflict precedence documentation in code)
- [x] 10.5 Write tests for Team circular reference check (self-reference, transitive cycle via RepoContext)
- [x] 10.6 Implement Team circular reference validator

## 11. Workflow

- [x] 11.1 Write tests for `Workflow` structural validator: id format, required fields, steps min 1, states min 2
- [x] 11.2 Implement `Workflow` struct and structural validator
- [x] 11.3 Write tests for Workflow semantic validators: states must have at least one `complete` and one non-complete; if any ReviewStep is present, at least one `reviewing` state required
- [x] 11.4 Implement Workflow states semantic validator
- [x] 11.5 Write tests for ReviewStep name uniqueness within steps list
- [x] 11.6 Implement ReviewStep name uniqueness validator
- [x] 11.7 Write tests for ReviewStep `on_reject` ordering (references earlier step, references later step, references unknown step)
- [x] 11.8 Implement ReviewStep `on_reject` ordering validator
- [x] 11.9 Write tests for WorkStep `depends_on` integrity (all names exist, unknown name fails)
- [x] 11.10 Implement WorkStep `depends_on` integrity validator
- [x] 11.11 Write tests for Workflow RACI and HooksMap referential integrity (role_ids and hook_ids exist in RepoContext)
- [x] 11.12 Implement RACI and hook invocation referential integrity validators for Workflow

## 12. Task

- [x] 12.1 Write tests for `Task` structural validator: id format, required fields, instructions/criteria min 1, states min 2
- [x] 12.2 Implement `Task` struct and structural validator
- [x] 12.3 Write tests for Task states semantic constraint (at least one `complete`, at least one non-complete; `reviewing` semantic rejected at type level)
- [x] 12.4 Implement Task states semantic validator
- [x] 12.5 Write tests for Task RACI and HooksMap referential integrity
- [x] 12.6 Implement RACI and hook invocation referential integrity validators for Task

## 13. Relay

- [x] 13.1 Write tests for `Relay` structural validator: id format, required fields, state_map min 1
- [x] 13.2 Implement `Relay` struct and structural validator
- [x] 13.3 Write tests for Relay `delegates_to` referential integrity (exists in shared/, unknown fails)
- [x] 13.4 Implement Relay `delegates_to` validator
- [x] 13.5 Write tests for Relay `state_map` key integrity (keys match shared workflow state names, unmapped states silently pass)
- [x] 13.6 Implement Relay `state_map` key integrity validator
- [x] 13.7 Write tests for Relay `state_map` semantic constraint (at least one `complete`, at least one non-complete)
- [x] 13.8 Implement Relay `state_map` semantic constraint validator
- [x] 13.9 Write tests for Relay RACI and HooksMap referential integrity
- [x] 13.10 Implement RACI and hook invocation referential integrity validators for Relay

## 14. Hook Invocation Input Validation

- [x] 14.1 Write tests for hook invocation input validator: required inputs present in `with`, no unknown keys allowed
- [x] 14.2 Implement hook invocation input validator (applies when resolving HooksMap entries against known Hook definitions in RepoContext)

## 15. Schema Alignment Verification

- [x] 15.1 For each entity, verify that JSON Schema fields and Rust struct fields are consistent in name, type, and constraints — document any discovered gaps as issues
