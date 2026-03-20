# Pari

**A workflow runtime for hybrid human-agent teams.**

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

## Built in Rust

Pari is lightweight, fast and runs as a live service.

---

## Learn More

- [Vision & Core Beliefs](docs/vision.md)
- [Who Is Pari For?](docs/who-is-pari-for.md)
