//! `RepoSlot` — encoding targets within markdown+YAML-frontmatter files.

use crate::substrate::pipeline::Slot;

#[derive(Clone, Copy)]
pub enum RepoSlot {
    /// The H1 heading line (`# Name`).
    H1,
    /// A named YAML frontmatter key.
    FrontmatterKey(&'static str),
    /// All YAML frontmatter keys not claimed by a `FrontmatterKey` slot
    /// (collects `x-*` extension keys).
    FrontmatterFlattened,
    /// First paragraph of the body (between H1 and first `##` section or EOF).
    DescriptionParagraph,
    /// Content under a `## Heading` section.
    Section(&'static str, SectionContent),
    /// Entire raw file content (for template files).
    FileContent,
}

#[derive(Clone, Copy)]
pub enum SectionContent {
    Paragraph,
    BulletList,
}

impl Slot for RepoSlot {}
