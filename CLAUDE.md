# Pari — Codebase Guide

Onboarding for Claude during active development in this repo. Authoritative for communication style, ways of working, workflows, and authoring guidelines. Design rules live in design docs and are binding — refer out, do not skip.

For the repo-wide convention map, see [docs/conventions.md](/Users/vinuth/code/pari/docs/conventions.md).

## What This Is

Rust library (package `pari-core`, crate name `pari`) for workflow runtime behavior in hybrid human-agent teams.

- Project pitch and the problem being solved: [README.md](/Users/vinuth/code/pari/README.md).
- Vision and the world being built for: [docs/vision.md](/Users/vinuth/code/pari/docs/vision.md).
- Intended end users: [docs/who-is-pari-for.md](/Users/vinuth/code/pari/docs/who-is-pari-for.md).
- Architectural reference (formal layers, ownership, dependency rules, within-layer structure): [docs/design/layers/layer-model.md](/Users/vinuth/code/pari/docs/design/layers/layer-model.md). Use that vocabulary when describing code ownership.

## Layer Map In Source

```text
src/
  entity/        entity-layer identity, plain entities, refs,
                 and tracked-field primitives                  -> see src/entity/CLAUDE.md
  workspace/     workspace-layer caller-facing async API,
                 viewer/editor handles, validation rules        -> see src/workspace/CLAUDE.md
  store/         store-layer server, state custodian,
                 dispatch boundaries                            -> see src/store/CLAUDE.md
  substrate/     substrate-layer persistence contracts/backends -> see src/substrate/CLAUDE.md
  error/         error-layer shared error infrastructure        -> see src/error/CLAUDE.md
  lib.rs         crate module wiring

pari-macros/
  proc-macro support for generated behavior across formal layers -> see pari-macros/CLAUDE.md

tests/
  integration coverage                                          -> see tests/CLAUDE.md
```

`schemas/` contains generated JSON Schema outputs for plain entity types. It is an output directory, not an architectural layer.

When working in a subtree, also look for a `CLAUDE.md` file in that directory or an ancestor within the repo. Treat nested guidance as additional local context.

## Working Preferences

- Treat design docs as authoritative unless a real implementation constraint forces a design amendment.
- Implement one task at a time. After each task, wait for the diff to be reviewed, and commit only once approved. Commits map to tasks so diffs stay easy to review task-by-task.
- Both implementation and tests follow the design. They are coordinate, not sequential — implementation does not dictate tests, tests do not dictate implementation, and design dictates both. Every change updates whichever are affected to stay aligned with the relevant design principles. Orthogonal or minor changes that don't affect behavior may not warrant test changes.
- If a test is awkward to write or pushes against the design, that is a design gap. Escalate as a design change rather than bending implementation or tests to work around it.

## TODO.md Lifecycle

A `TODO.md` is the working plan for a single change. It is scoped to a single git branch — created at branch start, deleted in the last commit of the branch when it is ready to merge.

- **Create** at the start of any sizable change. Structure the file as **Phases > Sections > Steps**, covering the change end to end: design, implementation, tests, build, deploy. Trivial changes (single-file edits, one-step fixes) don't need one.
- **Use** as the single queue for the change. Add new topics, open questions, and follow-ups into the active `TODO.md` as they surface.
- **Parallel work** is supported. One or more agents may work the list concurrently. Coordination among them is decided per change run, not statically prescribed.
- **Groom** when scope changes. Re-shape phases, surface anything that should escalate to a design doc, and remove outdated tasks outright — git tracks the history.
- **Absorb and delete** when the change is complete. Move durable artifacts — decisions, learnings, new conventions — into the appropriate docs (design docs, conventions, CLAUDE.md). Delete the `TODO.md` in the last commit of the branch.

This lifecycle will eventually be replaced by Pari's own workflow runtime once the runtime side of workflow execution lands; the authoring side already exists.

## Engineering Principles

- **DRY across code and docs.** A concept lives in exactly one place. Other places link to it. Applies equally to code (no duplicated logic) and documentation (no restated rules).
- **Articulative naming.** Names should explain themselves. When the right name isn't obvious, raise it with the developer rather than picking one in isolation — naming is a design decision worth a brief discussion.
- **Modularize, componentize, compose.** Treat every problem, at its granularity, as a candidate for a standalone, reusable component — one that could theoretically be published independently. Forces generic, loosely-coupled interfaces and prevents implicit coupling.
- **Folder and file structure encapsulate a single concept.** Each folder or file owns one concept at its granularity, grouping related items together. Don't scatter what belongs together.
- **Always present the current state.** Code, docs, and plans describe how things are now — not how they got here, what they used to be, or what was changed. No "previously this was X", no "renamed from Y", no commentary that assumes the reader knows prior versions. Git history is the canonical record of evolution; current artifacts are the canonical record of current state.

## Useful References

- Conventions index: [docs/conventions.md](/Users/vinuth/code/pari/docs/conventions.md)
- Architecture: [docs/design/layers/layer-model.md](/Users/vinuth/code/pari/docs/design/layers/layer-model.md)
- Design index: [docs/design/README.md](/Users/vinuth/code/pari/docs/design/README.md)
- Repository organization: [docs/design/repository.md](/Users/vinuth/code/pari/docs/design/repository.md)
- Root crate wiring: [src/lib.rs](/Users/vinuth/code/pari/src/lib.rs)
