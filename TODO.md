# TODO

Persistent queue for design-to-code drift cleanup. Work through these one task at a time and make one commit per completed task.

## Working Agreement

- Queue new topics here as they surface so they are not lost across sessions.
- Treat design as authoritative unless a real implementation constraint forces a design amendment.
- Complete code changes first, then fix tests afterward.
- Commit at the end of each completed task for easier per-task review.

## Code Tasks

1. [x] Entity identity core
   Context: `src/entity.rs` still models embedded entity identity in a workflow-specific way via `WorkflowParent { workflow_id }` and `EntityRef::new_embedded(id, workflow_id)`. The design docs now treat parentage as a generic identity concern expressed through `EntityRef<T, ParentKind>`, where parent relationships are part of semantic identity and not hard-coded to workflows in constructors.
   Goal: make `src/entity.rs` reflect the authoritative identity design completely. Remove workflow-specific constructor semantics, make parent representation match the design, and update helper APIs, hashing/equality assumptions, and serde accordingly.
   Scope: `src/entity.rs` only for this task. Do not yet update entity structs, proc-macros, or tests in this commit.
   Done looks like: the identity core in code can represent the designed parent-chain model without relying on `new_embedded(id, workflow_id)` or a workflow-id-only parent struct.

2. [x] Embedded entity types
   Context: once the identity core is fixed, embedded entities will still be wired to the old `WorkflowParent` shape and old constructor expectations. Their field types and surrounding code must follow the redesigned parent identity model.
   Goal: update the entity definitions for `Task`, `Relay`, and `EmbeddedWorkflow`, plus any directly related workflow step types, so they use the new identity core consistently and no longer assume the old workflow-id-only parent shape.
   Scope: `src/entities/task.rs`, `src/entities/relay.rs`, `src/entities/workflow.rs`, and closely related entity-source files only.
   Done looks like: entity structs compile against the new `EntityRef<T, ParentKind>` model and no entity source file still depends on the old embedded-constructor semantics.
   Completion note: after task 1, these entity source files were already aligned. No source edits were needed for this task; remaining `workflow_id` drift is outside the entity definition layer and is queued in later tasks.

3. [x] Entity proc-macros
   Context: the proc-macros still encode assumptions from the older identity design, including `WorkflowParent`-specific handling and generation paths that were built around the old embedded entity model.
   Goal: bring macro generation in line with the updated identity design so generated code no longer hard-codes workflow-specific parenting behavior.
   Scope: `pari-macros/src/lib.rs` and only the macro implementation pieces needed to support the new entity identity model.
   Done looks like: generated entity-related code can work with the new parent model without special-casing workflow-only semantics.

4. [x] Remove legacy repo substrate
   Context: `src/substrate/repo/` is a partially migrated legacy backend that mixes current substrate concepts with schema-era rendering, storage, and persistence behavior. It is not aligned with the latest design, and carrying it forward makes the substrate boundary harder to reason about. The direct tests and local docs depending on it are intentionally allowed to break for now.
   Goal: remove the repo substrate implementation from the crate so it can be re-designed cleanly later, instead of incrementally patching a heavily drifted backend.
   Scope: source code only for this task. Remove `src/substrate/repo/` from the module graph and delete its implementation files. Do not spend this task re-implementing a new backend or fixing tests/local docs.
   Done looks like: the crate no longer exposes `pari::substrate::repo`, the legacy repo substrate sources are deleted, and any resulting rebuild work is visible as explicit follow-up gaps rather than hidden behind old code.

5. [ ] Tracking framework cleanup
   Context: the current design does not treat `Tracked<T>`, `TrackedMap<K, V>`, or `#[derive(Tracked)]` as first-class concepts. The real tracking model is built around `TrackedField<T>` on tracked entities plus store-owned added/modified/removed state. The old generic tracking framework remains in code largely as legacy scaffolding.
   Goal: remove `Tracked<T>`, remove `TrackedMap<K, V>`, remove `#[derive(Tracked)]`, and simplify the codebase so `TrackedField<T>` is the only tracking primitive that remains aligned with the design.
   Scope: source code only for this task. Remove or refactor code that exists only to support the obsolete generic tracking framework, but do not do broad test cleanup in this commit.
   Done looks like: the code no longer depends on `Tracked<T>`, `TrackedMap<K, V>`, or `#[derive(Tracked)]`, and the remaining tracking model matches the design’s field-centric approach.

6. [ ] Accessor/setter generation and tracked-field usage
   Context: after the obsolete generic tracking framework is removed, the next source-side drift is the remaining tracked-field helper naming and accessor assumptions. Some generated code and source usage patterns still reflect older APIs and older access patterns.
   Goal: finish the source-side migration so generated accessors/setters and direct tracked-field usage follow the current tracked-field design and naming.
   Scope: source files only, including proc-macro-generated patterns where needed. Do not spend this task on test cleanup yet.
   Done looks like: the main code paths no longer depend on stale tracked-field/accessor APIs or their older semantics.

7. [ ] Single substrate boundary
   Context: the design and earlier cleanup discussions converged on a single substrate boundary in `src/substrate/mod.rs`, but the codebase and local guides still contain residual assumptions about a separate store-side substrate interface.
   Goal: make the source code consistently treat `src/substrate/mod.rs` as the only substrate boundary and remove remaining architectural drift from source modules.
   Scope: source modules only. This task is about architectural boundary cleanup, not yet signature-style cleanup or tests.
   Done looks like: the code no longer has meaningful source-level dependency on an old `store::Substrate` architecture.

8. [ ] Substrate trait signature cleanup
   Context: the current trait definition style and its implementations have drifted. The substrate trait uses `fn -> impl Future + Send`, but some implementations still use `async fn`, leaving the contract and its implementations inconsistent.
   Goal: make the substrate trait and all implementations use one coherent signature style that matches the chosen design direction.
   Scope: `src/substrate/mod.rs`, `src/store/mod.rs`, and repo substrate implementation files only.
   Done looks like: the trait and all its implementations express the same async contract style consistently.

9. [ ] Persist API migration
   Context: the code still carries older persistence naming and structure, especially `atomic_persist` and changeset-era terminology, even though the design and later code direction moved to `persist`.
   Goal: complete the source-side rename and structural migration from `atomic_persist`-era APIs to the current persist design.
   Scope: source files only, especially `src/substrate/repo/storage.rs`, `src/substrate/changeset.rs`, and any directly related modules.
   Done looks like: no source API that should now be `persist` still exposes or relies on old `atomic_persist` naming or stale changeset semantics.

10. [ ] Store internals alignment
   Context: `src/store/mod.rs` still contains several pockets of design drift in request/response handling, load and ensure-mutable flow, naming, and persist orchestration. Earlier tasks clear the prerequisites so this task can focus on the store actor itself.
   Goal: align the store internals with the current store design end-to-end, including message flow, response naming/shape, loading flow, ensure-mutable behavior, and persist plumbing.
   Scope: `src/store/mod.rs` and closely related store-internal source files only.
   Done looks like: the store implementation reads like the design docs rather than a carry-over from earlier TDD shortcuts.

11. [ ] Persist-path implementation constraint cleanup
    Context: even after API cleanup, the persist path may still have real Rust borrowing/lifetime friction because it needs to walk store-owned entity state while interacting with the substrate. This may be a pure code cleanup, or it may expose a legitimate design gap.
    Goal: refactor the persist path so the current design is implemented cleanly without brittle borrow workarounds. If a real constraint remains, stop and queue a focused design amendment rather than smuggling a workaround into code.
    Scope: only the persist-path implementation and the minimal source files needed to make it clean.
    Done looks like: either the persist path cleanly matches the current design, or a clearly scoped design-gap item is queued with the code left in a deliberately understandable state.

12. [ ] Code-local docs cleanup
    Context: several local guidance files still document old architecture and API shapes, which creates a strong risk of reintroducing drift in later sessions. After the code is corrected, these local docs need to be brought back in sync.
    Goal: update the code-local guidance files so they reflect the corrected source state and stop teaching stale patterns.
    Scope: local context docs only, such as `src/store/CLAUDE.md`, `src/entities/CLAUDE.md`, `src/error/CLAUDE.md`, `pari-macros/CLAUDE.md`, `tests/CLAUDE.md`, and any similarly scoped module guidance files.
    Done looks like: local docs match the code/design state after the code-cleanup tasks above.

## Test Tasks

13. [ ] Entity identity tests
    Context: after the code-side identity migration, many tests will still assume `WorkflowParent { workflow_id }` and `EntityRef::new_embedded(...)`.
    Goal: update tests to the new identity model without changing the design again.
    Scope: tests only.
    Done looks like: identity-related tests assert the generic parent-chain model rather than the old workflow-specific constructor model.

14. [ ] Tracked primitive tests
    Context: once tracked semantics and tracked-field usage are fixed in source, tests will still reflect the older dirty-state default and stale helper names.
    Goal: update tests to the current tracked design.
    Scope: tests only.
    Done looks like: tests assert clean-on-create tracked semantics and current tracked-field APIs.

15. [ ] Persist/storage tests
    Context: the storage and substrate tests still encode `atomic_persist` and older persistence behavior.
    Goal: update those tests to the current persist API and behavior after the source migration is done.
    Scope: tests only.
    Done looks like: persist/storage tests reflect the current API names and behavior rather than legacy naming.

16. [ ] Macro/entity generation tests
    Context: proc-macro and generated-API tests will drift after the identity and accessor changes land.
    Goal: update tests so they reflect the corrected generated API shape.
    Scope: tests only.
    Done looks like: generated entity/macro behavior is asserted against the new source-of-truth model.

17. [ ] Error tests
    Context: the codebase already has settled error-handling changes that can be reflected in tests, even before we return to the larger Activity and higher-layer error redesign.
    Goal: update tests only for the error behavior that is already settled by current design and code-cleanup work, without expanding scope into the next round of error design.
    Scope: tests only.
    Done looks like: tests no longer enforce stale error assumptions that we already know are wrong, while unresolved higher-layer error design remains intentionally out of scope.

18. [ ] Full test sweep
    Context: after the targeted code and test cleanup tasks above, the remaining failures become signal rather than noise.
    Goal: run the full test suite, classify any remaining failures, and split them into true code bugs versus real design gaps that need discussion.
    Scope: full verification only.
    Done looks like: the remaining queue is smaller, clearer, and based on real unresolved issues instead of accumulated drift.
