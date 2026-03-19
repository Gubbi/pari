## Context

Pari is a new project with no existing codebase. The context is the vision established in the proposal: a workflow runtime for hybrid human-agent teams, built in Rust, exposed via MCP.

The primary challenge is designing a system that is simultaneously:
- **Flexible** — teams define their own workflows, not Pari
- **Enforcing** — workflow rules are structural, not advisory
- **Agent-native** — AI agents check in with Pari as a live runtime, not a document

Stakeholders: Those who own how the team works (define workflows), contributors and AI agents (operate within them).

## Goals / Non-Goals

**Goals:**
- Any team can define their own workflow without needing to understand Pari's internals
- AI agents and human contributors check in with Pari and receive authoritative, up-to-date guidance — what's next, what's blocking, and why
- Workflow gates and sequences are enforced deterministically — not suggested
- The team's "why" — behind each task and stage — is stored and delivered as a first-class part of the check-in loop
- Pari runs as a standalone service with minimal operational overhead

**Non-Goals:**
- Prescribing any specific workflow methodology
- Owning domain standards (architecture, tooling, craft practices)
- Replacing project management tools (Linear, Jira, GitHub Issues)

## Decisions

### 1. Rust as the implementation language

**Decision:** Build Pari in Rust.

**Rationale:** The goal was something lightweight, fast, and easy to install — a service that feels unobtrusive. Rust's ability to ship a self-contained binary with no runtime dependencies makes installation simple and the footprint small. It's also fast and memory-safe. Whether another language would have served equally well hasn't been rigorously evaluated.

**Alternatives considered:** No formal alternatives analysis was done at this stage. The choice was driven by the goal of minimal installation and fast performance. Go or TypeScript could plausibly work for this use case and may be revisited as requirements become clearer.

---

### 2. MCP as the agent interface

**Decision:** Expose Pari as an MCP (Model Context Protocol) server.

**Rationale:** MCP is an open, agent-agnostic protocol. Any MCP-compatible agent — Claude, Cursor, custom agents — can interact with Pari without Pari needing per-agent integrations. Both authoring tools (define your workflow) and runtime tools (check in, query state) are exposed through this single interface.

**Alternatives considered:**
- REST API: HTTP works fine for programmatic access, but a REST endpoint definition doesn't help an agent understand how to use it. MCP's tool definition format — name, description, parameters, schema — is better positioned to help agents structure their calls and shields users from needing to orchestrate the interaction manually.
- Custom protocol: would limit adoption without a clear upside; MCP is becoming a standard.

---

### 3. Team-authored workflows via conversational authoring

**Decision:** Workflows are defined by teams using their preferred AI agent + Pari's MCP authoring tools — not by filling out config files.

**Rationale:** Implicit team norms need to be drawn out through conversation, not transcribed from memory. A team lead talking with an agent that calls Pari's authoring tools produces richer, more accurate workflow definitions than a YAML file ever would.

**Alternatives considered:**
- Pure YAML/config: creates a blank-page problem for teams that haven't already formalized their process. It also isn't well-suited to capturing the narrative rationale behind workflow stages — the "why" context that agents need — in the way a conversation-driven approach can. A team lead describing their workflow out loud, with an agent asking clarifying questions, surfaces context that a YAML form never would.
- Web UI: too heavy for initial scope; the conversational skill + MCP is sufficient.

---

### 4. Unified accountability model for humans and agents

**Decision:** Roles in Pari are not typed as "human" or "agent" — they are roles with accountabilities, and either humans or agents may fill them.

**Rationale:** The team of the future is hybrid. Making the accountability model neutral at the primitive level means Pari doesn't need to be updated as the ratio of humans to agents shifts. A role has standards, guardrails, and tracked work — who fills it is incidental.

---

### 5. Live runtime with persistent workflow state

**Decision:** Pari is a running service, not a document store. It maintains real-time workflow state that agents query.

**Rationale:** File-based gating can handle simple prerequisites — OpenSpec demonstrates this, requiring a proposal before a spec, a spec before tasks. But enforcing gates that depend on runtime state — what's been completed, what's currently in progress, whether concurrent agents are working in conflict — requires something that holds live state. Pari's gates need to answer "is this work allowed to proceed right now?" not just "does this artifact exist?" That's the boundary a live runtime serves.

## Risks / Trade-offs

| Risk | Mitigation |
|------|------------|
| MCP protocol is still maturing | Pin to a specific MCP version; abstract the transport layer so we can swap |
| Workflow authoring is high-friction | The conversational authoring skill lowers the barrier; provide sensible defaults and starter templates |
| Enforcement that is too strict alienates contributors | Teams define their own gates — Pari enforces what the team chose, not what Pari prescribes |
| Persistent state adds operational burden | Pari should embed its state store (e.g., SQLite) — no external DB required to run |
| Agent diversity (shapes, sizes, accountabilities) | Role model is type-neutral; agent variety is handled at the role definition layer, not the runtime layer |

## Open Questions

- How does Pari handle teams that want to evolve their workflow while it's actively in use — and what happens to work already in progress under the previous version?
- What's the right model for exceptions — moments when a team legitimately needs to work outside their defined workflow without abandoning the workflow entirely?

*Implementation questions (persistence format, concurrency, MVP shape, versioning) will be addressed in subsequent design iterations.*
