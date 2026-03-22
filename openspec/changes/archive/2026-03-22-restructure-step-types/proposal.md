## Why

`Step` and `WorkStep` are currently concrete types where `WorkStep` hardcodes `definition: WorkStepDefinition`. `SharedStep` and `SharedWorkStep` are entirely separate parallel types that mirror `Step`/`WorkStep` with a different definition enum. This duplication makes the type hierarchy harder to maintain and blocks the `#[derive(Tracked)]` macro in the incremental-persistence change — the macro cannot derive tracked variants for `WorkStepDefinition` because its `Workflow(Box<WorkflowDef<Step>>)` variant requires recursive generic type inference inside a `Box`.

Making `Step<S>` and `WorkStep<S>` generic over the definition type collapses the parallel hierarchy and gives `WorkStepDefinition`/`SharedWorkStepDefinition` concrete `Box<Workflow>` and `Box<SharedWorkflow>` variants — plain type names the derive macro can handle with a simple `Tracked` prefix rule.

## What Changes

- `WorkStep` becomes `WorkStep<S>` with `definition: S` (generic over the step definition type)
- `Step` becomes `Step<S>` with `Work(WorkStep<S>)` | `Review(ReviewStep)` variants
- `WorkStepDefinition::Workflow(Box<WorkflowDef<Step>>)` becomes `Workflow(Box<Workflow>)`
- `SharedWorkStepDefinition::SharedWorkflow(Box<WorkflowDef<SharedStep>>)` becomes `SharedWorkflow(Box<SharedWorkflow>)`
- `WorkflowDef<S>` generic parameter `S` now represents the step definition type (not the step type): `steps: Vec<Step<S>>`
- Type aliases updated: `Workflow = WorkflowDef<WorkStepDefinition>`, `SharedWorkflow = WorkflowDef<SharedWorkStepDefinition>`
- `SharedStep` and `SharedWorkStep` concrete types replaced with type aliases: `type SharedStep = Step<SharedWorkStepDefinition>`, `type SharedWorkStep = WorkStep<SharedWorkStepDefinition>`

## Capabilities

### Modified Capabilities
- `workflow-step-types`: Step and WorkStep become generic; SharedStep/SharedWorkStep become type aliases; definition enums use concrete Workflow/SharedWorkflow in Box variants

## Impact

- **Modified**: `src/schema/entities/workflow.rs` — type definitions and impls
- **Updated**: `src/substrate/repo/render.rs` — pattern matches on step types
- **Updated**: `src/schema/validation.rs` — step traversal
- **Updated**: `tests/storage_integration.rs`, `tests/schema_coherence.rs` — step construction
- **Schema impact**: None — serde and schemars target plain structs which remain structurally identical; JSON output is unchanged
- **Prerequisite for**: `incremental-persistence`
