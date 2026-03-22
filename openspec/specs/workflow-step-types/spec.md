## ADDED Requirements

### Requirement: Step and WorkStep are generic over the step definition type
The system SHALL define `WorkStep<S>` with `definition: S` and `Step<S>` as `Work(WorkStep<S>) | Review(ReviewStep)` in `src/schema/entities/workflow.rs`. The type parameter `S` represents the step definition type. `WorkflowDef<S>` SHALL have `steps: Vec<Step<S>>`.

`SharedStep` and `SharedWorkStep` SHALL be type aliases:
- `type SharedStep = Step<SharedWorkStepDefinition>`
- `type SharedWorkStep = WorkStep<SharedWorkStepDefinition>`

The top-level workflow type aliases SHALL be:
- `type Workflow = WorkflowDef<WorkStepDefinition>`
- `type SharedWorkflow = WorkflowDef<SharedWorkStepDefinition>`

#### Scenario: Workflow construction uses generic Step<S>
- **WHEN** a `Workflow` is constructed with a `WorkStep` carrying a `WorkStepDefinition::Task`
- **THEN** the type is `WorkflowDef<WorkStepDefinition>` and the step is `Step::<WorkStepDefinition>::Work(WorkStep { definition: WorkStepDefinition::Task(...) })`

#### Scenario: SharedWorkflow construction uses Step<SharedWorkStepDefinition> via alias
- **WHEN** a `SharedWorkflow` is constructed with a `SharedWorkStep` carrying a `SharedWorkStepDefinition::Task`
- **THEN** `SharedStep` and `SharedWorkStep` aliases resolve correctly and construction is identical in shape to the non-shared case

---

### Requirement: WorkStepDefinition and SharedWorkStepDefinition use concrete Box types
`WorkStepDefinition::Workflow` SHALL carry `Box<Workflow>` (not `Box<WorkflowDef<Step>>`). `SharedWorkStepDefinition::SharedWorkflow` SHALL carry `Box<SharedWorkflow>` (not `Box<WorkflowDef<SharedStep>>`).

#### Scenario: Inline workflow nested inside WorkStepDefinition
- **WHEN** a `WorkStepDefinition::Workflow(Box::new(inner_wf))` is constructed where `inner_wf: Workflow`
- **THEN** the type of the boxed value is `WorkflowDef<WorkStepDefinition>` (via the `Workflow` alias)

#### Scenario: Inline shared workflow nested inside SharedWorkStepDefinition
- **WHEN** a `SharedWorkStepDefinition::SharedWorkflow(Box::new(inner_swf))` is constructed where `inner_swf: SharedWorkflow`
- **THEN** the type of the boxed value is `WorkflowDef<SharedWorkStepDefinition>` (via the `SharedWorkflow` alias)

---

### Requirement: JSON schema structure is unchanged; titles may differ
The restructure SHALL produce no change to the logical structure of the generated JSON schemas in `schemas/`. Serde and schemars continue to target the same logical structure — the generics are an internal Rust type-level concern only. Schema titles (the `title` field in generated schemas) may differ from pre-restructure output due to schemars incorporating generic type parameter names (e.g. `"Step"` may become `"Step_for_WorkStepDefinition"`); this is acceptable and expected.

#### Scenario: cargo xtask produces structurally identical schemas before and after
- **WHEN** `cargo xtask` is run after the restructure
- **THEN** the generated schema files in `schemas/` are structurally identical to the pre-restructure output (field shapes, constraints, and discriminators unchanged); schema `title` values may differ due to generic type parameter names
