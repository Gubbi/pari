# TODO

Persistent queue for design-to-code drift cleanup. Work through these one task at a time and make one commit per completed task.

## Working Agreement

- Queue new topics here as they surface so they are not lost across sessions.
- Treat design as authoritative unless a real implementation constraint forces a design amendment.
- Complete code changes first, then fix tests afterward.
- Commit at the end of each completed task for easier per-task review.
- For tests: remove irrelevant tests that only defend deleted architecture. Keep future-relevant broken tests disabled with a `TODO:` comment until the needed implementation exists.

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

5. [x] Remove legacy schema module
   Context: after removing the repo substrate, the remaining schema-era architecture was mostly isolated to `src/schema/`, `src/substrate/changeset.rs`, and a substrate re-export of the legacy `schema::store::EntityStore`. This stack duplicates the newer entity/store design and is the main reason the obsolete tracking framework still exists.
   Goal: delete the legacy schema module and the schema-based changeset layer from the crate so future persistence work builds on the current entity/store architecture instead of on duplicated legacy models.
   Scope: source code only for this task. Remove `src/schema/` from the crate, delete `src/substrate/changeset.rs`, and remove schema-based exports in source. Do not re-implement replacement fixtures/tests/local docs in this commit.
   Done looks like: the library no longer exposes `pari::schema`, no source module depends on `src/substrate/changeset.rs`, and the remaining migration gaps are surfaced explicitly in tests and fixtures rather than hidden behind legacy code.

6. [x] Tracking framework cleanup
   Context: the current design does not treat `Tracked<T>`, `TrackedMap<K, V>`, or `#[derive(Tracked)]` as first-class concepts. The real tracking model is built around `TrackedField<T>` on tracked entities plus store-owned added/modified/removed state. The old generic tracking framework remains in code largely as legacy scaffolding.
   Goal: remove `Tracked<T>`, remove `TrackedMap<K, V>`, remove `#[derive(Tracked)]`, and simplify the codebase so `TrackedField<T>` is the only tracking primitive that remains aligned with the design.
   Scope: source code only for this task. Remove or refactor code that exists only to support the obsolete generic tracking framework, but do not do broad test cleanup in this commit.
   Done looks like: the code no longer depends on `Tracked<T>`, `TrackedMap<K, V>`, or `#[derive(Tracked)]`, and the remaining tracking model matches the design’s field-centric approach.
   Completion note: the runtime tracking layer now keeps only `TrackedField<T>`, and it lives in `src/tracked/tracked_field.rs`. The old `Tracked` derive entrypoint is gone, but `pari-macros/src/lib.rs` still contains dead helper code that should be deleted in a follow-up cleanup.

7. [x] Proc-macro dead-code cleanup
   Context: after splitting the live proc-macros into dedicated files, the crate root still carries dead helper code left behind from the removed `Tracked` derive. It does not affect behavior now, but it is drift and will confuse future work.
   Goal: remove the obsolete tracked-derive helper block from `pari-macros/src/lib.rs` so the proc-macro crate contains only live entrypoints plus their supporting modules.
   Scope: `pari-macros/src/lib.rs` and, if useful, small supporting refactors within `pari-macros/src/`.
   Done looks like: `pari-macros/src/lib.rs` is a thin entrypoint only, with no dead tracked-derive helpers remaining.
   Completion note: this is already true after the proc-macro split. `pari-macros/src/lib.rs` now contains only the live entrypoints and module declarations.

8. [x] Accessor/setter generation and tracked-field usage
   Context: the code is now centered on `TrackedField<T>`, but generated accessors/setters and some surrounding expectations still reflect older helper naming and older mutation/loading assumptions.
   Goal: finish the source-side migration so generated accessors/setters and direct tracked-field usage follow the current tracked-field design and naming consistently.
   Scope: source files only, including proc-macro-generated patterns where needed. Do not spend this task on broader test cleanup.
   Done looks like: the main code paths no longer depend on stale tracked-field helper assumptions or older access semantics.
   Completion note: the live source no longer contains stale tracked-field helper usage; remaining hits were only in local guide docs. The meaningful code drift here was the public `EntityClient` boundary still living under `store/` and still carrying operation-facing errors there. This task moved `EntityClient` and its operation errors into a new `workspace/` module, updated generated accessor/setter paths to call `::pari::workspace::EntityClient`, and kept channel/request failures mapped into the operation-specific `StoreUnavailable(...)` variants so accessor/setter-triggered load/ensure-mutable flows remain non-panicking at the client boundary.

9. [ ] Substrate boundary alignment
   Context: the legacy repo substrate and schema stack are gone, leaving a clearer substrate boundary in `src/substrate/mod.rs`. The remaining code should now be aligned to that single boundary explicitly.
   Goal: make the source code consistently treat `src/substrate/mod.rs` as the only substrate boundary and remove any remaining architectural assumptions from source modules.
   Scope: source modules only. This task is about boundary cleanup, not yet implementing a concrete backend.
   Done looks like: the source no longer has meaningful architectural drift around substrate boundaries.

10. [ ] Substrate trait contract cleanup
   Context: the substrate traits and implementations still need consistency around async style and call shape. Now that legacy backends are removed, that contract can be cleaned up without compatibility baggage.
   Goal: make the substrate trait and all remaining implementations express one coherent async contract style that matches the design.
   Scope: `src/substrate/mod.rs`, `src/store/mod.rs`, and any in-tree substrate implementations.
   Done looks like: the substrate contract is internally consistent and no stale signature style remains.

11. [ ] Concrete substrate replacement
   Context: removing the legacy repo substrate intentionally left the project without a real filesystem-backed substrate. This is now the biggest functional gap rather than a drift-hiding problem.
   Goal: design and implement a new design-aligned concrete substrate that can replace the removed repo backend without reintroducing legacy architecture.
   Scope: new source implementation plus the minimal integration points needed to make it usable from the store.
   Done looks like: the project has a real concrete substrate again, built on the current design rather than on the deleted schema/repo stack.

12. [ ] Store internals alignment
    Context: `src/store/mod.rs` still contains pockets of design drift in request/response handling, loading flow, ensure-mutable behavior, naming, and persist orchestration. Earlier cleanup removed legacy distractions so this can now be addressed directly.
    Goal: align the store internals with the current design end-to-end, including message flow, response naming/shape, loading flow, ensure-mutable behavior, and persist plumbing.
    Scope: `src/store/mod.rs` and closely related store-internal source files only.
    Done looks like: the store implementation reads like the design docs rather than like a carry-over from earlier TDD shortcuts.
    Note: this task includes eliminating `StoreEntity` naming/abstraction drift. The design already uses `TrackedEntity` for the type-erased tracked wrapper role, so the code should align to that design concept instead of keeping `StoreEntity` as a parallel abstraction.

13. [ ] Persist-path implementation cleanup
    Context: even after API cleanup, the persist path may still have real Rust borrowing/lifetime friction because it needs to walk store-owned entity state while interacting with the substrate. This may be pure code cleanup, or it may expose a real design gap.
    Goal: refactor the persist path so the current design is implemented cleanly without brittle borrow workarounds. If a real constraint remains, stop and queue a focused design amendment instead of smuggling a workaround into code.
    Scope: only the persist-path implementation and the minimal source files needed to make it clean.
    Done looks like: either the persist path cleanly matches the current design, or a clearly scoped design-gap item is queued with the code left in a deliberately understandable state.

14. [ ] Code-local docs cleanup
    Context: several local guidance files still describe removed architecture such as schema, repo substrate, and the old tracked macro behavior. They now drift from both code and design.
    Goal: update the code-local guidance files so they reflect the corrected source state and stop teaching stale patterns.
    Scope: local context docs only, such as `src/store/CLAUDE.md`, `src/entities/CLAUDE.md`, `src/error/CLAUDE.md`, `src/substrate/CLAUDE.md`, `pari-macros/CLAUDE.md`, `tests/CLAUDE.md`, and similar module guidance files.
    Done looks like: local docs match the code/design state after the code-cleanup tasks above.

## Test Tasks

15. [ ] Test surface normalization
    Context: the obviously legacy tests have been deleted, and some future-relevant tests are now intentionally disabled with file-level `TODO`s. The remaining test surface still needs to be normalized so it represents only meaningful current or future coverage.
    Goal: remove any remaining irrelevant tests, keep future-relevant broken tests disabled with clear `TODO`s, and leave only meaningful unit/integration/end-to-end test files active.
    Scope: tests only. This is a pruning/normalization task, not yet a full rewrite of all broken tests.
    Done looks like: the test tree contains only meaningful current/future tests, and disabled files explain exactly what implementation gap blocks re-enabling them.

16. [ ] Entity identity and tracked-field test updates
    Context: after the identity and tracking cleanup, the still-relevant tests must be updated to the current `EntityRef` and `TrackedField` APIs. Some are already disabled because they still expect removed helpers like `new_initialized` or `with_value`.
    Goal: update relevant tests to the new identity model and the current `TrackedField` API.
    Scope: tests only, especially `entity_definitions`, `validate_entities`, `tracked_serde`, and `derive_entity`.
    Done looks like: identity- and tracking-related tests assert the current design and no longer rely on removed helper APIs.

17. [ ] Substrate and end-to-end test restoration
    Context: `core_jobs` and substrate-focused tests were disabled because the legacy concrete backend was removed. Once a new substrate exists, these tests should come back in a design-aligned form.
    Goal: restore meaningful substrate-level integration tests and end-to-end job tests against the new concrete substrate.
    Scope: tests only, after the concrete substrate replacement exists.
    Done looks like: the project again has meaningful end-to-end and substrate integration coverage without reviving legacy repo/schema assumptions.

18. [ ] Error and macro test updates
    Context: the codebase already has settled identity/macro/error changes that can be reflected in tests even before returning to the next round of higher-layer error design.
    Goal: update tests only for the error and macro behavior that is already settled by current design and code cleanup, without expanding scope into undecided error architecture.
    Scope: tests only.
    Done looks like: tests no longer enforce stale macro/error assumptions that we already know are wrong, while unresolved higher-layer error design remains intentionally out of scope.

19. [ ] Full test sweep
    Context: after the targeted code and test tasks above, the remaining failures become signal rather than noise.
    Goal: run the full test suite, classify any remaining failures, and split them into true code bugs versus real design gaps that need discussion.
    Scope: full verification only.
    Done looks like: the remaining queue is smaller, clearer, and based on real unresolved issues instead of accumulated drift.
