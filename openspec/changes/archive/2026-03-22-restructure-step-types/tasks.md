## 1. Type Definitions

- [x] 1.1 Write tests for generic WorkStep<S> and Step<S>: Workflow construction with WorkStepDefinition, SharedWorkflow construction via type aliases
- [x] 1.2 Make WorkStep generic: change WorkStep { definition: WorkStepDefinition } to WorkStep<S> { definition: S }
- [x] 1.3 Make Step generic: change Step enum to Step<S> with Work(WorkStep<S>) | Review(ReviewStep)
- [x] 1.4 Update WorkStepDefinition::Workflow variant from Box<WorkflowDef<Step>> to Box<Workflow>
- [x] 1.5 Update SharedWorkStepDefinition::SharedWorkflow variant from Box<WorkflowDef<SharedStep>> to Box<SharedWorkflow>
- [x] 1.6 Update WorkflowDef<S> steps field from Vec<S> to Vec<Step<S>>; update type aliases to Workflow = WorkflowDef<WorkStepDefinition> and SharedWorkflow = WorkflowDef<SharedWorkStepDefinition>
- [x] 1.7 Replace SharedStep and SharedWorkStep concrete types with type aliases: type SharedStep = Step<SharedWorkStepDefinition>, type SharedWorkStep = WorkStep<SharedWorkStepDefinition>

## 2. Downstream Updates

- [x] 2.1 Update src/schema/validation.rs: fix all step pattern matches and traversal for new generic types
- [x] 2.2 Update src/substrate/repo/render.rs: fix all step pattern matches for new generic types
- [x] 2.3 Update tests/storage_integration.rs: fix step construction and helper functions for new types
- [x] 2.4 Update tests/schema_coherence.rs if any step type references need updating

## 3. Verification

- [x] 3.1 Run cargo test — all existing tests pass with no logic changes
- [x] 3.2 Run cargo xtask — generated schemas in schemas/ are identical to pre-restructure output
