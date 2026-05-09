# Pari — Conventions

A reverse index of where conventions, decisions, and durable rules live in this repo. Each entry lists the *categories of content* in that file, not every rule — open the file for specifics.

If a bullet list under a file grows past ~8 items, that's a signal to split the file.

## How to use this index

Looking for the rule on something? Scan the bullet lists below until a category matches, then open that file. The index is keyed by file path; topics live in their files.

## Product context

### README.md (repo root)
- Project pitch and problem statement
- High-level overview of what Pari does
- Top-level entry point for visitors

### docs/vision.md
- The world Pari is being built for
- Hybrid human-agent team thesis

### docs/who-is-pari-for.md
- Intended end users — the "champion" persona
- Frustrations and contexts that surface Pari

## Starting points

### CLAUDE.md (repo root)
- First-read orientation for Claude during active work in this repo
- Authoritative source for communication style, ways of working, workflows, and authoring guidelines during interactive development sessions in this repo
- Internal-developer-facing — distinct from CONTRIBUTING.md, which governs external contribution workflow
- Strong orientation toward design docs as binding for design principles, not optional guidance — refer out, do not skip or restate
- Pointers into design docs and per-layer CLAUDE.md files for everything outside its own authority

### AGENTS.md (repo root)
- Entry point for non-Claude agents
- Pointer to CLAUDE.md as the agent reference and to this file as the starting point
- Non-Claude-specific agent guidance

### docs/conventions.md (this file)
- Reverse index of where conventions live
- Doc-split meta-rule (which file owns which kind of content)

## Design docs — sources of truth

### docs/design/README.md
- Design directory entry point
- C4 model alignment (which level lives where)
- TOC of all design docs

### docs/design/framework.md
- L2 Container view of Pari
- Client and persistence extension seams
- Core layer roles and error hierarchy for integrators
- Runtime independence requirement

### docs/design/layers/layer-model.md
- Formal layer vocabulary (entity, workspace, store, substrate, error). Validation lives inside `workspace` as a sub-area.
- Dependency rules between layers
- Pure (`lib/`) vs orchestration (layer root) split
- Structural conventions (`mod.rs` scope, naming)

### docs/design/layers/*.md (per-layer)
- Per-layer L3 component design
- Layer-specific types, flows, invariants
- Local naming and ownership rules

### docs/design/repository.md
- Workspace shape and members
- Published vs internal vs binary-distributed crates
- Library name (`pari`) vs package name (`pari-core`)
- Lockstep versioning rooted at `pari-core`
- Stable-Rust requirement and rationale
- Distribution mechanisms (crates.io, cargo-dist, Docker, npm)

### docs/design/test.md
- Testing strategy and black-box principles
- Three-tier coverage funnel
- Test layout and fixture style
- Substrate parameterization

### docs/design/CLAUDE.md
- Authoring guidance for design docs
- Source-of-truth rules (L2/L3 in design/, L4 in rustdoc, CLAUDE.md is derived)
- Style requirements (no meta-commentary, visual richness)

## Per-area onboarding (CLAUDE.md tree)

### src/<layer>/CLAUDE.md
- Quick onboarding for an agent working in that layer
- Local invariants and gotchas not captured in the layer's design doc
- Strong orientation to the layer's design doc as authoritative — refer out, do not restate

### tests/CLAUDE.md
- Quick onboarding for the integration test crate
- Local helpers and entry points
- Strong orientation to test.md as authoritative for testing strategy and fixture style

### pari-macros/CLAUDE.md
- Quick onboarding for proc-macro development
- Macro-specific conventions and constraints
- Pointers to layer design docs whose generated code lives here

## Doc-split meta-rule

Three (eventually four) docs serve overlapping audiences. This rule decides what belongs where.

| Doc | Audience | Test for inclusion |
|---|---|---|
| `docs/conventions.md` (this file) | Anyone, first read | "Where does X live?" Reverse index only — no rules of its own beyond the meta-rule below. |
| `docs/design/*.md` | Anyone | "Is this a durable architectural rule or design decision?" Source of truth for L2 (framework) and L3 (per-layer) design. Repo-level decisions go in `repository.md`. |
| `CLAUDE.md` (root + per-area) | Claude during active work | "Is this how this repo conducts active development, or how to onboard into an area fast?" Authoritative for communication style, ways of working, workflows, and authoring guidelines. Internal-developer-facing — distinct from CONTRIBUTING.md. Refers out to design docs for design principles, never restates them. |
| `AGENTS.md` | Non-Claude agents | "Is this agent guidance that isn't Claude-specific?" Mostly a pointer file; substantive content is in CLAUDE.md and conventions.md. |
| `CONTRIBUTING.md` (future) | External contributors | "Is this PR workflow, gates, pre-work, or AI-contribution expectations?" External-facing onboarding. Created when external contributions are anticipated. |

Two corollaries:

- **Design docs win conflicts.** If `CLAUDE.md` and a design doc disagree, the design doc is authoritative and the `CLAUDE.md` is stale. Fix the `CLAUDE.md`.
- **No duplication.** A rule lives in exactly one place. Other docs link to it. If you find yourself restating a rule, move the rule to the appropriate design doc and link.
