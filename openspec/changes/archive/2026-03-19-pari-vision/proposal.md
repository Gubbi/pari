## Why

As AI agents become first-class participants in how teams work, teams lack infrastructure to define, enforce, and evolve how work actually happens. Workflow alignment is left to prompts (which drift) and documentation (which nobody reads), with no deterministic way to hold humans and agents to the same standards.

## What Changes

Pari is a new project — a workflow runtime for hybrid human-agent teams. It introduces:

- A **team-authored workflow model**: teams define their own stages, gates, roles, artifacts, standards, and the "why" behind each stage
- A **live runtime** that enforces the workflow deterministically — gates actually gate, sequences actually sequence
- An **MCP server** as the primary interface: agents check in on a loop to know what's next, what's blocking, what the context is, and the why behind their task
- A **unified accountability model**: humans and agents are held to the same standards within the defined workflow
- An **authoring experience** via skill/command: team leads use their preferred AI agent + Pari's MCP tools to define their workflow conversationally
- Built in **Rust** as deployable infrastructure

Pari is explicitly **not prescriptive** about methodology — teams define their own process. In software development this might be SDD, TDD, DDD, vibe coding, or blends; in other domains the vocabulary will differ. Pari is also not a project manager (not Linear, Jira, GitHub Issues) — it owns process norms, not domain-specific standards or task tracking.

## Capabilities

### New Capabilities

- `workflow-definition`: How teams author, store, and evolve their workflow — stages, gates, roles, artifacts, standards, and the "why" of each stage
- `agent-runtime`: The check-in loop interface — agents query Pari for what's next, what's blocking, current context, and the why behind their task
- `accountability`: Defining and tracking standards/metrics for roles (human or agent), with guardrails and work tracking
- `mcp-server`: The MCP server exposing both authoring and runtime tools to any MCP-compatible agent
- `workflow-enforcement`: Deterministic gate and guardrail evaluation — not suggestive, structural

### Modified Capabilities

*(none — this is a new project)*

## Impact

- New Rust project; no application code exists yet
- MCP protocol as the primary agent interface (agent-agnostic by design)
- Primary champion: Anyone on a team — at any level — who feels the friction of misaligned ways of working and wants it structurally solved
