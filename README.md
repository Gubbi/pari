# Pari

**A workflow runtime for hybrid human-agent teams.**

[![CI](https://github.com/Gubbi/pari/actions/workflows/ci.yml/badge.svg)](https://github.com/Gubbi/pari/actions/workflows/ci.yml)
[![codecov](https://codecov.io/github/Gubbi/pari/graph/badge.svg?token=9H5UAW59O9)](https://codecov.io/github/Gubbi/pari)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

> **Status: pre-1.0.** APIs, on-disk layout, and entity shapes are unstable. Expect breaking changes between commits until the first tagged release.

Pari (ಪರಿ — *way, manner, method*) is the infrastructure layer that lets teams define how they work — and actually enforces it, for everyone on the team, including AI agents.

---

## The Problem

As AI agents become first-class participants in how teams work, teams are discovering that prompts don't scale. Context drifts. Agents go off-script. New team members — human or agent — spend weeks absorbing implicit norms that were never written down. The team's way of working exists only in people's heads, and it erodes a little every day.

Pari is the answer to that erosion.

---

## What Pari Does

Teams use Pari to **define** their workflow — the tasks, reviews, roles, artifacts, and the *why* behind each step. Then Pari **enforces** it, deterministically, for everyone operating within it.

AI agents don't just receive context about how the team works. They **check in with Pari** — asking what's next, what's blocking, what the task is really for. Pari answers authoritatively, based on the workflow the team defined.

```
Team defines:    stages, gates, roles, artifacts, the "why"
        │
        ▼
    Pari Runtime
        │
        ├── Contributors and their agent asks:    "what's next?"       → Pari answers
        ├── Autonomous agent asks:                "am I allowed to?"   → Pari answers
        └── Leads and their agent asks:           "what's the state?"  → Pari answers
```

---

## What Makes Pari Different

- **Not prescriptive.** Pari doesn't tell you to use SDD, TDD, DDD, or any other methodology. You define how your team works. Pari holds it.
- **Not a project manager.** Pari doesn't replace Linear, Jira, or GitHub Issues. It owns *process norms*, not task tracking.
- **Agents as team members.** Pari treats AI agents with the same accountability model as humans — roles, standards, guardrails.
- **Enforcement with teeth.** Workflow rules are structural, not advisory. Gates gate. Sequences sequence.

---

## Who It's For

Pari is for anyone who has felt the friction of a team — human and agent — not working the way it should. The person who cares that alignment actually holds, not just that it's documented. That concern can live at any level of a team, in any kind of organization.

→ See [who is Pari for](docs/who-is-pari-for.md) for a fuller picture.

---

## A Concrete Example

Consider a small design team's workflow — every change goes through *task → review → ship*, with a reviewer who isn't the author. The team encodes that as a Pari workflow once. From then on, every contributor and agent operating in the workspace is held to it.

The bundled `RepoSubstrate` lays the workflow out as a directory of Markdown files — entity bodies in prose, structured fields in YAML frontmatter:

```text
my-team/
├── workflows/
│   └── design-flow/
│       ├── README.md            # workflow: states, steps (Design + Review), RACI, intercepts
│       └── draft/
│           ├── README.md        # task entity: assignee role, briefing, gates
│           └── template.md      # artifact template the task produces
└── common/
    ├── roles/
    │   ├── designer.md
    │   └── reviewer.md
    └── artifact-kinds/
        └── design-doc.md        # schema + validation for the artifact
```

Once that's loaded, anyone on the team — or their agent — gets authoritative answers from the runtime:

```
contributor's agent → Pari: "what's next on 2026-redesign-nav?"
              Pari → agent: "step `draft` is open; you are the assignee;
                            blocking artifact is design-doc."

reviewer's agent  → Pari: "can I approve while I'm still listed as the author?"
              Pari → agent: "no — `approve` requires role `reviewer`,
                            and the author cannot self-review."
```

The workflow lives in the substrate, not in prompts. The same questions get the same answers tomorrow, next quarter, and after the team rotates.

> A runnable version of this example will live under `examples/` once the runtime side of execution lands.

---

## Architecture

Pari is organised into formal layers — *entity*, *workspace*, *store*, *substrate* — with strict ownership and dependency rules. Entities are plain data; the workspace is the caller-facing async API; the store custodies state and dispatches to backends; the substrate is the persistence seam.

→ Start with the [layer model](docs/design/layers/layer-model.md), then the [design index](docs/design/README.md).

---

## Building Locally

Pari is a Rust library (`pari-core`, crate name `pari`). Standard cargo:

```sh
cargo build --workspace
cargo test --workspace
cargo xtask generate-schemas    # regenerate schemas/*.json from entity types
```

CI runs the same on every push; schema drift fails the build.

---

## Project Status

Pre-1.0. Not yet published to crates.io. The crate is usable as a path or git dependency for experimentation; expect API churn until the first tagged release.

Library test coverage (`pari-core` only): **87% lines, 83% regions** at the time of writing. CI publishes coverage on every push.

---

## Contributing

Issues and discussions are welcome. A `CONTRIBUTING.md` with development conventions and PR guidelines will land separately.

In the meantime, the working norms for this repo are documented in [CLAUDE.md](CLAUDE.md) and the [conventions index](docs/conventions.md).

---

## License

MIT. See [LICENSE](LICENSE).

---

## Learn More

- [Vision & core beliefs](docs/vision.md)
- [Who is Pari for?](docs/who-is-pari-for.md)
- [Design index](docs/design/README.md)
- [Workspace organization](docs/design/repository.md)
