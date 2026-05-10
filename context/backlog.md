# Backlog

A short next-up queue (typically 3–5 items) of confirmed high-priority items not currently being worked on. Not an exhaustive list of ideas — only items the team has decided to pick up next. Entries are removed when a branch starts addressing them, or when explicitly dropped.

Items are grouped by area for grep-ability.

## Testing

### Improve test coverage

Library coverage on `pari-core` is **87% lines / 83% regions**. The headline number is healthy, but it masks two specific gaps that should be closed before we treat coverage as in steady state.

- **`RepoSubstrate` is under-exercised by parameterized tests.** Per [docs/design/test.md](../docs/design/test.md) *Substrate parameterization*, integration tests are meant to run against every concrete substrate, with backend-specific tests layered on top. Coverage tells a different story:
  - [src/substrate/repo/lib/executor.rs](../src/substrate/repo/lib/executor.rs): 30.5% lines
  - [src/substrate/repo/substrate.rs](../src/substrate/repo/substrate.rs): 47.6% lines
  - [src/substrate/repo/resolver.rs](../src/substrate/repo/resolver.rs): 80.0% lines

  Investigate why filesystem paths aren't being hit the way in-memory paths are — whether the parameterization is silently skipping `RepoSubstrate`, whether the tests it does run are too narrow, or whether substrate-specific tests are missing the error branches.

- **Validation rules without paired pass/fail coverage.** Every validation rule should have at least one fixture that passes the rule and one that fails it. Coverage suggests several rules don't:
  - [src/workspace/validation/lib/rules/cross_entity/workflow.rs](../src/workspace/validation/lib/rules/cross_entity/workflow.rs): 43.9% lines
  - [src/workspace/validation/lib/rules/structural/hook.rs](../src/workspace/validation/lib/rules/structural/hook.rs): 75.0% lines / 51.5% regions
  - [src/workspace/validation/lib/schema.rs](../src/workspace/validation/lib/schema.rs): 57.4% lines

  Codify the pass-and-fail-fixture-per-rule expectation in [docs/design/test.md](../docs/design/test.md), then audit existing rules against it and close the gaps.

## Documentation

### Convert design-doc cross-tree references to root-absolute paths

Design docs under `docs/design/` currently use relative paths (`../../src/...`, `../framework.md`) for cross-tree references. The convention going forward is **root-absolute `/`-prefixed paths** for cross-tree references (short same-directory hops stay bare). [docs/design/git-workflow.md](../docs/design/git-workflow.md) already follows the new style.

Sweep the existing docs (`framework.md`, `repository.md`, `test.md`, `layers/*.md`, `CLAUDE.md` files in `docs/design/`) and convert. Mechanical change; one-shot branch.
