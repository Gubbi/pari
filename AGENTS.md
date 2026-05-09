# Pari — Agent Entry Point

Entry point for non-Claude agents (Codex, Cursor, etc.) working in this repo. Claude Code auto-loads `CLAUDE.md`; this file is the equivalent surface for agents that follow the AGENTS.md convention.

The same documentation governs all agents — agent identity does not change the rules. The pointers below apply uniformly.

## Where to start

1. **[docs/conventions.md](docs/conventions.md)** — reverse index of every doc, keyed by file path. Use it to navigate the repo.
2. **[CLAUDE.md](CLAUDE.md)** — authoritative for onboarding, communication style, ways of working, workflows, and authoring guidelines during active development. Apply its rules regardless of which agent reads it.
3. **[docs/design/](docs/design/)** — authoritative for design principles. Binding, not optional.

When working in a subtree, also look for a `CLAUDE.md` file in that directory or an ancestor within the repo. Treat nested guidance as additional local context.
