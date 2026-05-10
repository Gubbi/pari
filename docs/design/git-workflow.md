# Git Workflow

How work moves through this repository — branches, commits, merges, and the CI gates that verify every push.

## Branching

`main` is always shippable. Direct commits to `main` are not allowed. Every change — including a one-line typo fix — lands through a branch.

Branches are named `<area>/<short-slug>`:

| Area | When |
|---|---|
| `feat` | New behavior or capability |
| `fix` | Bug fix |
| `docs` | Documentation only |
| `refactor` | Code reshaping without behavior change |
| `chore` | Tooling, CI, dependencies |
| `test` | Test additions or restructuring |
| `perf` | Performance work |

Examples: `docs/readme-overhaul`, `feat/workspace-validators`, `fix/token-leak`.

Branches are deleted on origin after merge.

### Long-lived branches are an antipattern

Long-lived means **large**, not slow. A branch that sits open for months is fine if its eventual diff is small and self-contained — life happens, contributors come back to a branch when they have bandwidth. A branch that *grows* without merging is not.

The size test: can a reviewer hold the whole change in their head in one sitting? If not, split the branch into independent vertical slices, each itself a complete end-to-end change — design, implementation, and tests together. Valid axes for splitting:

- **Refactor before feature.** A no-behavior-change reshape lands on `main` first; the feature builds on the new shape.
- **Migration before cleanup.** Move callers to the new path; remove the old path in a follow-up.
- **Standalone scaffolding before the work that consumes it.** If a new helper, fixture set, or abstraction has value on its own, it can ship on its own.

A commit and a branch are end-to-end threads of one small logical change — with the tests that prove the change works. Never split along the test/implementation/design axis.

## Commits

One commit per reviewed task. Each commit is a self-contained, reviewable unit — it builds, it passes tests, and it tells one story.

### Commit messages

A commit message has two responsibilities: **say what this commit changes** and **say why it exists in the context of the branch**.

| Field | Rule |
|---|---|
| **Subject** | Imperative, under ~70 characters, names the intent. ("Add MIT LICENSE", not "Added a license file".) |
| **Body** | The **why** (the context this commit lives in — what goal it serves, what constraint forced it) and the **delta** (what this commit moves forward toward that goal). Wrap at ~72 columns. |

Never enumerate file-level changes — `git diff` is the canonical record of *what* changed. The commit message owns *why* and *toward what*.

## Rebasing

Branches stay rebased on `main`. Merging `main` into a branch is not allowed — it pollutes the linear history and obscures the branch's actual delta.

Force-push (`git push --force-with-lease`) is fine on your own branch. Never on `main`.

## Merging

Branches merge into `main` via **squash-on-merge**. The squash collapses the branch's commit history into a single commit on `main`.

The squash commit's message must capture the **cumulative intent and delta of the branch** — not a concatenation of the per-commit messages, and not a cherry-list of "did A, then B, then C". Read the branch as one change and write the message that change deserves.

After merging, the branch is deleted on origin:

```sh
git push origin --delete <branch>
```

Local stale branches are pruned with `git fetch --prune` periodically.

## Build and Verification Pipeline

Three CI jobs run in parallel on every push and pull request. The workflow lives at [`.github/workflows/ci.yml`](/.github/workflows/ci.yml); this section is the source of truth for what it must do.

| Job | Command | Gate |
|---|---|---|
| `test` | `cargo test --workspace` | Library + integration suite passes. |
| `schemas` | `cargo xtask generate-schemas` then `git diff --exit-code schemas/` | Committed `schemas/` match what the entity types regenerate to. Drift fails the build. |
| `coverage` | `cargo llvm-cov --package pari-core --lcov` | Coverage scoped to `pari-core`; `xtask` and `pari-macros` are excluded from the headline number. The Codecov upload step runs only on `main`; the instrument-and-run step runs on every branch so a future merge gate can assert a coverage floor without paying for upload. |

`xtask` is the codegen entry point for the workspace. Today it exposes `generate-schemas` (writes `schemas/*.json` from entity types via `JsonSchema`) and `check-schemas` (asserts structural invariants over the generated files). New build-time tooling lives in `xtask` before any of it gets reified into a Cargo plugin or external binary.

### Cost discipline

- `paths-ignore` skips runs when only Markdown, `docs/**`, `context/**`, or `LICENSE` change — pure-docs commits don't exercise cargo.
- A `concurrency` group keyed on `${{ github.workflow }}-${{ github.ref }}` cancels a superseded run when a fresh push lands on the same ref.
