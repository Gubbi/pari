# Test

The `test` layer covers everything the project does to verify its own
behavior. It is governed by one philosophy applied at every scope:
**black-box tests, no mocks, real implementations**. Granularity
varies — unit, integration, functional — but the style does not.

The framework-level view is in [../framework.md](../framework.md). The
layering rules are in [layer-model.md](layer-model.md). This document
covers the L3 design of the test layer: principles, the coverage
funnel, the three tiers and what each is for, layout, and the
substrate strategy for functional tests.

## Two Principles

| Principle | What it means |
|---|---|
| Black-box scope | A test sees only the public surface of the component(s) it exercises. No reaching into private fields, no `pub(crate)` test-only hooks, no asserting on internal collections. |
| No mocks | Substitutes are real, peer implementations of the same trait — picked for environment, not for behavior shaping. `InMemorySubstrate` is not a mock of `RepoSubstrate`; it is another backend that satisfies the same `Substrate` contract. |

These apply identically at every tier. A unit test for a pure
cycle-detection function asserts on its public input/output the same
way a functional test asserts on the result of a user-visible
operation.

## No Zealous Coverage

Tests track real, plausible failure modes — not every theoretically
reachable rare scenario. It is a legitimate outcome for a tier to be
empty. The point of the funnel below is *not* to require tests at
every level; it is to describe where coverage naturally lands.

## Coverage Funnel

```text
       ┌──────────────────────────────┐  ← most surface coverage lives here
       │   Functional (per user job)  │     /tests/functional/
       └──────────────────────────────┘
         ┌──────────────────────────┐    ← only what isn't reachable from above
         │  Integration (per comp.) │       /tests/integration/
         └──────────────────────────┘
           ┌──────────────────────┐      ← logic-heavy units, future-proof assumptions
           │   Unit (colocated)   │         in `src/`, `#[cfg(test)] mod tests`
           └──────────────────────┘
```

This inverts the classical pyramid deliberately. Functional tests
exercise the layer composition the user actually pays for; that is
where the bulk of behavior coverage belongs. Integration and unit
tests fill specific gaps the functional layer cannot reach.

## Three Tiers

| Tier | Why this tier exists (indicative, not exhaustive) |
|---|---|
| **Functional** | The behavior the user pays for, observable end-to-end through the public API. The default home for any new test. |
| **Integration** | Pins behavior at composition boundaries that functional tests cannot reach without contortion — e.g. store-unavailable on the workspace ↔ store seam (channel closed mid-operation), or a partial substrate response that exercises the store ↔ substrate merge path in isolation. The examples are illustrative; any genuine boundary-failure mode that resists end-to-end coverage is a candidate. |
| **Unit** | Three distinct reasons, all valid: (1) **logic-heavy pure functions** — cycle detection, state-map validation, ref collection, parser primitives — where black-box input/output coverage of the function itself is the cleanest assertion; (2) **future-proof assumptions** — capabilities a unit promises to honor that current callers do not fully exercise, pinned so future edits cannot silently undercut them; (3) **combinatorial coverage** — pure functions with input spaces large enough that exhausting them through a higher tier would balloon the test surface, often paired with property or parameterized testing. |

Empty tiers are fine. If functional coverage already pins everything
worth pinning, the integration and unit folders may stay empty.

## Layout

| Tier | Path | Grouping |
|---|---|---|
| Unit | `src/<layer>/<file>.rs` — `#[cfg(test)] mod tests` in the same file | The function/type under test |
| Integration | `tests/integration/<composition>.rs` | One composition — any combination of layers/seams useful for the boundary-failure case being pinned |
| Functional | `tests/functional/<user_job>.rs` | One user job per file; multiple scenarios per file. Filenames are ad-hoc and named as user jobs appear (`author_workflow.rs`, `check_in_changes.rs`, etc.) |
| Fixtures | `tests/fixtures/<entity>.rs` | One file per entity kind. Owns builders and canonical sample data only — no assertion helpers, no setup orchestration. |

### Cargo Wiring

Cargo treats every `tests/*.rs` file as its own integration binary,
each linked against the library independently. All non-unit tests
land in a single binary at `tests/tests.rs` so that link is paid
once.

`tests/tests.rs` declares the layout's directories as submodule trees
via `#[path]`:

```rust
#[path = "common/mod.rs"]    mod common;
#[path = "fixtures/mod.rs"]  mod fixtures;
#[path = "functional/mod.rs"] mod functional;
// #[path = "integration/mod.rs"] mod integration;  // added when populated
```

Each directory's `mod.rs` declares its child files in turn
(`pub mod author_role;`, etc.), so the tier paths in the table above
are the on-disk layout the binary actually loads. Shared helpers
live under `tests/common/`.

## Substrate Strategy For Functional Tests

`RepoSubstrate` is the only end-user backend currently shipped, so it
is the canonical functional substrate. Functional tests run against
`RepoSubstrate` over a tempdir.

Today there is also `InMemorySubstrate`, a peer backend used during
development. To establish the multi-substrate test path *now* — so
that adding a new backend in the future is a known, cheap cost
rather than a design surprise — functional tests are written so that
the substrate is a parameter, and at least one scenario per user job
runs against both `RepoSubstrate` and `InMemorySubstrate`.

This is not a force-fit: substrate-specific scenarios are allowed
where they make sense. A scenario that only matters for a filesystem
backend (cleanup of `.part` directories, permission edge cases) is
fine to keep `RepoSubstrate`-only. A scenario that only matters for
ephemeral state would similarly stay `InMemorySubstrate`-only. The
parameter-and-twice rule applies to scenarios where the substrate is
incidental to the behavior under test.

## What This Rules Out

- Mocks of any production trait.
- `pub(crate)` or `pub(in ...)` test-only hooks added to peek at
  private state.
- Assertions on private collections, private fields, or internal call
  counts.
- Manufactured rare-scenario tests that do not pin a real assumption
  or capability.
- Integration tests that duplicate coverage already provided by
  functional tests.

## What This Does Not Rule Out

- Real substrate over a tempdir, with the test inspecting the files
  `RepoSubstrate` writes — that is still observation of the public
  contract.
- Integration-tier fault injection via a hand-written `Substrate`
  implementation that returns specific errors. Such an impl is a
  *peer* of the production substrates, not a mock — it satisfies the
  same trait, exists for environment reasons (deterministic failure),
  and never inspects the caller.
- Pinning a future-proof assumption with a unit test even when no
  current caller exercises it.

## Implications For Production Code

Every seam a test wants to swap must already be a public trait with
at least one production peer. If a test is tempted to reach into
private state, the bug is the missing seam, not the test. New
production seams are added when tests motivate them; tests do not
adapt to insufficient seams by reaching in.

