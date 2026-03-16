# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Overview

This repository uses **OpenSpec** — a spec-driven development workflow. All feature work flows through the OpenSpec CLI and a structured change lifecycle. There is no application code here; the repo is a workspace for managing changes via OpenSpec artifacts.

## OpenSpec CLI Commands

```bash
openspec new change "<name>"                        # Scaffold a new change
openspec list --json                                # List all changes
openspec status --change "<name>" --json            # Get artifact status for a change
openspec instructions <artifact-id> --change "<name>" --json  # Get instructions for creating an artifact
```

## Workflow (Slash Commands)

| Command | Description |
|---|---|
| `/opsx:explore` | Think through ideas before committing to a change |
| `/opsx:propose <name>` | Create a change and generate all artifacts (proposal, design, tasks) |
| `/opsx:apply <name>` | Implement tasks from a change |
| `/opsx:archive <name>` | Archive a completed change |

## Repository Structure

```
openspec/
  config.yaml           # Schema config (currently: spec-driven)
  changes/              # Active changes, each containing:
    <name>/
      .openspec.yaml
      proposal.md       # What & why
      design.md         # How
      tasks.md          # Implementation steps (- [ ] / - [x])
      specs/            # Delta specs (capability overrides)
    archive/            # Completed changes (moved here by /opsx:archive)
  specs/                # Main project specs by capability
.claude/
  commands/opsx/        # Slash command definitions
  skills/               # Skill implementations for each workflow action
```

## Spec-Driven Schema Artifact Order

Artifacts are created in dependency order. The `applyRequires` field in `openspec status --json` tells you which artifacts must exist before implementation can start. For the **spec-driven** schema that is typically: `proposal` → `design` → `tasks`.

When implementing (`/opsx:apply`), mark each task checkbox `- [ ]` → `- [x]` immediately after completing it.

## Key Behaviors

- **`context` and `rules`** from `openspec instructions` output are constraints for the AI — never copy them into artifact files.
- **`template`** from `openspec instructions` output is the structure to follow when writing an artifact.
- Delta specs in `openspec/changes/<name>/specs/` override main specs; they get synced to `openspec/specs/` during archive.
- Archive target name format: `YYYY-MM-DD-<change-name>`.
