# Design Docs — Authoring Guidance

Instructions for writing and editing docs in this directory.

## Write for a fresh-mind reader

A design doc is a snapshot of the current design. Write so a reader with no prior context can pick it up cold.

- No meta-commentary about how the design arrived at its current state.
- No references to recent changes, rewrites, or "previously this was X".
- No conversational or project-history framing.
- No "note to reader" asides explaining the doc's own structure.

Design docs are not changelogs, diaries, or commit messages. State the design in the present tense and move on.

## Visual richness

Use Mermaid diagrams, ASCII art, and tables liberally. A reader should be able to understand a section's shape from its visuals before reading the prose.

- Prefer diagrams over long narrative descriptions of structure.
- Prefer tables over nested bullet lists for comparative information.
- Reference source by `file:line` rather than duplicating code inline.
