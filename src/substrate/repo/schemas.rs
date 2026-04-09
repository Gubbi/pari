//! `SubstrateSchema<RepoSubstrate>` implementations for all entity types.

use crate::substrate::pipeline::{
    AssetDef, EntitySchema, FieldMapping, RefAssetDef, SubstrateSchema, MARKDOWN_FILE, RAW_FILE,
};
use super::slot::{RepoSlot, SectionContent};
use super::RepoSubstrate;
use crate::entities::{
    artifact_kind::ArtifactKind,
    hook::Hook,
    relay::Relay,
    role::Role,
    task::Task,
    team::Team,
    workflow::{EmbeddedWorkflow, ReusableWorkflow, Workflow},
};

impl SubstrateSchema<RepoSubstrate> for Role {
    const SCHEMA: EntitySchema<RepoSlot> = EntitySchema {
        ref_asset: RefAssetDef {
            path_template: "roles/{id}.md",
            kind: &MARKDOWN_FILE,
            fields: &[
                FieldMapping { key: "name",        slot: RepoSlot::H1 },
                FieldMapping { key: "description", slot: RepoSlot::DescriptionParagraph },
                FieldMapping { key: "purpose",     slot: RepoSlot::FrontmatterKey("purpose") },
                FieldMapping { key: "traits",      slot: RepoSlot::FrontmatterKey("traits") },
                FieldMapping { key: "extensions",  slot: RepoSlot::FrontmatterFlattened },
            ],
        },
        assets: &[],
    };
}

impl SubstrateSchema<RepoSubstrate> for Hook {
    const SCHEMA: EntitySchema<RepoSlot> = EntitySchema {
        ref_asset: RefAssetDef {
            path_template: "hooks/{id}.md",
            kind: &MARKDOWN_FILE,
            fields: &[
                FieldMapping { key: "name",         slot: RepoSlot::H1 },
                FieldMapping { key: "description",  slot: RepoSlot::DescriptionParagraph },
                FieldMapping { key: "instructions", slot: RepoSlot::Section("Instructions", SectionContent::BulletList) },
                FieldMapping { key: "inputs",       slot: RepoSlot::FrontmatterKey("inputs") },
                FieldMapping { key: "extensions",   slot: RepoSlot::FrontmatterFlattened },
            ],
        },
        assets: &[],
    };
}

impl SubstrateSchema<RepoSubstrate> for Team {
    const SCHEMA: EntitySchema<RepoSlot> = EntitySchema {
        ref_asset: RefAssetDef {
            path_template: "teams/{id}.md",
            kind: &MARKDOWN_FILE,
            fields: &[
                FieldMapping { key: "name",        slot: RepoSlot::H1 },
                FieldMapping { key: "description", slot: RepoSlot::DescriptionParagraph },
                FieldMapping { key: "members",     slot: RepoSlot::FrontmatterKey("members") },
                FieldMapping { key: "include",     slot: RepoSlot::FrontmatterKey("include") },
                FieldMapping { key: "import",      slot: RepoSlot::FrontmatterKey("import") },
                FieldMapping { key: "extensions",  slot: RepoSlot::FrontmatterFlattened },
            ],
        },
        assets: &[],
    };
}

impl SubstrateSchema<RepoSubstrate> for ArtifactKind {
    const SCHEMA: EntitySchema<RepoSlot> = EntitySchema {
        ref_asset: RefAssetDef {
            path_template: "artifact-kinds/{id}.md",
            kind: &MARKDOWN_FILE,
            fields: &[
                FieldMapping { key: "name",        slot: RepoSlot::H1 },
                FieldMapping { key: "description", slot: RepoSlot::DescriptionParagraph },
                FieldMapping { key: "service",     slot: RepoSlot::FrontmatterKey("service") },
                FieldMapping { key: "access",      slot: RepoSlot::FrontmatterKey("access") },
                FieldMapping { key: "guidance",    slot: RepoSlot::Section("Guidance", SectionContent::Paragraph) },
                FieldMapping { key: "extensions",  slot: RepoSlot::FrontmatterFlattened },
            ],
        },
        assets: &[],
    };
}

impl SubstrateSchema<RepoSubstrate> for Workflow {
    const SCHEMA: EntitySchema<RepoSlot> = EntitySchema {
        ref_asset: RefAssetDef {
            path_template: "workflows/{id}/README.md",
            kind: &MARKDOWN_FILE,
            fields: &[
                FieldMapping { key: "name",        slot: RepoSlot::H1 },
                FieldMapping { key: "description", slot: RepoSlot::DescriptionParagraph },
                FieldMapping { key: "purpose",     slot: RepoSlot::Section("Purpose", SectionContent::Paragraph) },
                FieldMapping { key: "raci",        slot: RepoSlot::FrontmatterKey("raci") },
                FieldMapping { key: "steps",       slot: RepoSlot::FrontmatterKey("steps") },
                FieldMapping { key: "states",      slot: RepoSlot::FrontmatterKey("states") },
                FieldMapping { key: "intercepts",  slot: RepoSlot::FrontmatterKey("intercepts") },
                FieldMapping { key: "guidance",    slot: RepoSlot::Section("Guidance", SectionContent::Paragraph) },
                FieldMapping { key: "extensions",  slot: RepoSlot::FrontmatterFlattened },
            ],
        },
        assets: &[],
    };
}

impl SubstrateSchema<RepoSubstrate> for ReusableWorkflow {
    const SCHEMA: EntitySchema<RepoSlot> = EntitySchema {
        ref_asset: RefAssetDef {
            path_template: "reusable-workflows/{id}/README.md",
            kind: &MARKDOWN_FILE,
            fields: &[
                FieldMapping { key: "name",        slot: RepoSlot::H1 },
                FieldMapping { key: "description", slot: RepoSlot::DescriptionParagraph },
                FieldMapping { key: "purpose",     slot: RepoSlot::Section("Purpose", SectionContent::Paragraph) },
                FieldMapping { key: "raci",        slot: RepoSlot::FrontmatterKey("raci") },
                FieldMapping { key: "steps",       slot: RepoSlot::FrontmatterKey("steps") },
                FieldMapping { key: "states",      slot: RepoSlot::FrontmatterKey("states") },
                FieldMapping { key: "intercepts",  slot: RepoSlot::FrontmatterKey("intercepts") },
                FieldMapping { key: "guidance",    slot: RepoSlot::Section("Guidance", SectionContent::Paragraph) },
                FieldMapping { key: "extensions",  slot: RepoSlot::FrontmatterFlattened },
            ],
        },
        assets: &[],
    };
}

impl SubstrateSchema<RepoSubstrate> for EmbeddedWorkflow {
    const SCHEMA: EntitySchema<RepoSlot> = EntitySchema {
        ref_asset: RefAssetDef {
            path_template: "{parent.base}/{id}/README.md",
            kind: &MARKDOWN_FILE,
            fields: &[
                FieldMapping { key: "name",        slot: RepoSlot::H1 },
                FieldMapping { key: "description", slot: RepoSlot::DescriptionParagraph },
                FieldMapping { key: "purpose",     slot: RepoSlot::Section("Purpose", SectionContent::Paragraph) },
                FieldMapping { key: "raci",        slot: RepoSlot::FrontmatterKey("raci") },
                FieldMapping { key: "steps",       slot: RepoSlot::FrontmatterKey("steps") },
                FieldMapping { key: "states",      slot: RepoSlot::FrontmatterKey("states") },
                FieldMapping { key: "intercepts",  slot: RepoSlot::FrontmatterKey("intercepts") },
                FieldMapping { key: "guidance",    slot: RepoSlot::Section("Guidance", SectionContent::Paragraph) },
                FieldMapping { key: "extensions",  slot: RepoSlot::FrontmatterFlattened },
            ],
        },
        assets: &[],
    };
}

impl SubstrateSchema<RepoSubstrate> for Task {
    const SCHEMA: EntitySchema<RepoSlot> = EntitySchema {
        ref_asset: RefAssetDef {
            path_template: "{parent.base}/{id}/README.md",
            kind: &MARKDOWN_FILE,
            fields: &[
                FieldMapping { key: "name",         slot: RepoSlot::H1 },
                FieldMapping { key: "description",  slot: RepoSlot::DescriptionParagraph },
                FieldMapping { key: "purpose",      slot: RepoSlot::Section("Purpose", SectionContent::Paragraph) },
                FieldMapping { key: "instructions", slot: RepoSlot::Section("Instructions", SectionContent::BulletList) },
                FieldMapping { key: "criteria",     slot: RepoSlot::Section("Criteria", SectionContent::BulletList) },
                FieldMapping { key: "artifact",     slot: RepoSlot::FrontmatterKey("artifact") },
                FieldMapping { key: "raci",         slot: RepoSlot::FrontmatterKey("raci") },
                FieldMapping { key: "states",       slot: RepoSlot::FrontmatterKey("states") },
                FieldMapping { key: "intercepts",   slot: RepoSlot::FrontmatterKey("intercepts") },
                FieldMapping { key: "guidance",     slot: RepoSlot::Section("Guidance", SectionContent::Paragraph) },
                FieldMapping { key: "extensions",   slot: RepoSlot::FrontmatterFlattened },
            ],
        },
        assets: &[AssetDef {
            path_template: "{parent.base}/{id}/template.md",
            kind: &RAW_FILE,
            fields: &[FieldMapping { key: "template_content", slot: RepoSlot::FileContent }],
            path_deps: &[],
        }],
    };
}

impl SubstrateSchema<RepoSubstrate> for Relay {
    const SCHEMA: EntitySchema<RepoSlot> = EntitySchema {
        ref_asset: RefAssetDef {
            path_template: "{parent.base}/{id}/README.md",
            kind: &MARKDOWN_FILE,
            fields: &[
                FieldMapping { key: "name",         slot: RepoSlot::H1 },
                FieldMapping { key: "description",  slot: RepoSlot::DescriptionParagraph },
                FieldMapping { key: "purpose",      slot: RepoSlot::Section("Purpose", SectionContent::Paragraph) },
                FieldMapping { key: "raci",         slot: RepoSlot::FrontmatterKey("raci") },
                FieldMapping { key: "delegates_to", slot: RepoSlot::FrontmatterKey("delegates_to") },
                FieldMapping { key: "briefing",     slot: RepoSlot::Section("Briefing", SectionContent::Paragraph) },
                FieldMapping { key: "debriefing",   slot: RepoSlot::Section("Debriefing", SectionContent::Paragraph) },
                FieldMapping { key: "state_map",    slot: RepoSlot::FrontmatterKey("state_map") },
                FieldMapping { key: "intercepts",   slot: RepoSlot::FrontmatterKey("intercepts") },
                FieldMapping { key: "guidance",     slot: RepoSlot::Section("Guidance", SectionContent::Paragraph) },
                FieldMapping { key: "extensions",   slot: RepoSlot::FrontmatterFlattened },
            ],
        },
        assets: &[],
    };
}
