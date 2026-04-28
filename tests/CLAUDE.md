# tests — Functional Coverage

Cross-cutting test layer. Single integration binary at
[tests.rs](/Users/vinuth/code/pari/tests/tests.rs) declares
[`common/`](/Users/vinuth/code/pari/tests/common),
[`fixtures/`](/Users/vinuth/code/pari/tests/fixtures), and
[`functional/`](/Users/vinuth/code/pari/tests/functional) via `#[path]`
so the link cost is paid once.

Authoritative design doc: [docs/design/test.md](/Users/vinuth/code/pari/docs/design/test.md).
When this file and the design doc disagree, the design doc wins.

## Local Orientation

- Substrate parameterization (`SubstrateKind`, `run_with`):
  [common/substrate.rs](/Users/vinuth/code/pari/tests/common/substrate.rs).
- Per-entity fixtures (named constructor functions; one file per
  entity kind): [fixtures/](/Users/vinuth/code/pari/tests/fixtures).
- Functional tests, one file per user job:
  [functional/](/Users/vinuth/code/pari/tests/functional).

## User Jobs Currently Covered

| File | Job |
|---|---|
| `author_role.rs` | Author a role. |
| `author_team.rs` | Author a team, including roster and composition. |
| `author_workflow.rs` | Author a top-level workflow with embedded children. |
| `author_workflow_with_intercepts.rs` | Author a workflow with lifecycle intercepts. |
| `author_embedded_workflow.rs` | Author a workflow with a nested embedded workflow. |
| `author_reusable_workflow.rs` | Author a reusable workflow. |
| `author_relay.rs` | Author a workflow with a relay step. |
| `modify_persisted_entity.rs` | Modify a previously-persisted entity. |
| `validation_failures.rs` | Validation rejects invalid input across all three tiers. |
| `validation_timing.rs` | Validation fires at the right lifecycle point (setter / commit). |
| `lifecycle_failures.rs` | Lifecycle preconditions are enforced (duplicate insert, missing-entity ops, checkout collisions, persist with pending). |

## Conventions Worth Repeating Locally

- Fixtures are named constructor functions (`a_minimal_role(id)`,
  `a_role_with_traits(id)`), not builders. Variants compose internally
  from smaller partial helpers.
- Black-box tests, no mocks. `InMemorySubstrate` is a peer of
  `RepoSubstrate`, not a mock — at least one scenario per
  substrate-incidental user job runs against both via `run_with`.
- For substrates whose persisted artifacts are user-observable
  (`RepoSubstrate` files), assert on the on-disk shape alongside the
  API result. Single-file backends use the singular
  `repo_substrate_writes_expected_<entity>_file`; directory-tree
  backends use the plural `…_files` to reflect that more than one
  file is checked.
- Workflows are authored iteratively to satisfy parent-existence and
  ref-existence cross-entity invariants at every transaction boundary
  — insert empty-steps shell first, then embedded children, then
  modify steps to the final shape.
- Prefer `EntityClient::checkout(EntityRef::<X>::new(id))` (typed) for
  mutation paths and `EntityClient::resolve(any_ref)` (type-erased)
  for read-only assertions. Setters and `commit(self)` /
  `undo_checkout(self)` are reachable only through the typed delegate
  returned by checkout.
