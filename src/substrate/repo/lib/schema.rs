use crate::{
    entities::{
        artifact_kind::ArtifactKind,
        hook::Hook,
        relay::Relay,
        role::Role,
        task::Task,
        team::Team,
        workflow::{EmbeddedWorkflow, ReusableWorkflow, Workflow},
    },
    substrate::{
        pipeline::{
            AssetDef, AssetKind, EntitySchema, FieldMapping, RefAssetDef, Slot, SubstrateSchema,
        },
        RepoSubstrate,
    },
};

#[derive(Clone, Copy)]
pub enum RepoSlot {
    H1,
    FrontmatterKey(&'static str),
    FrontmatterFlattened,
    DescriptionParagraph,
    Section(&'static str, SectionContent),
    FileContent,
}

impl Slot for RepoSlot {}

#[derive(Clone, Copy)]
pub enum SectionContent {
    Paragraph,
    BulletList,
}

const MARKDOWN_FILE: &AssetKind = &AssetKind {
    distinguishes_create: false,
    supports_partial: false,
};
const RAW_FILE: &AssetKind = &AssetKind {
    distinguishes_create: false,
    supports_partial: false,
};
const EMPTY_ASSETS: &[AssetDef<RepoSlot>] = &[];
const ROLE_FIELDS: &[FieldMapping<RepoSlot>] = &[
    FieldMapping {
        key: "name",
        slot: RepoSlot::H1,
    },
    FieldMapping {
        key: "description",
        slot: RepoSlot::DescriptionParagraph,
    },
    FieldMapping {
        key: "purpose",
        slot: RepoSlot::FrontmatterKey("purpose"),
    },
    FieldMapping {
        key: "traits",
        slot: RepoSlot::FrontmatterKey("traits"),
    },
    FieldMapping {
        key: "extensions",
        slot: RepoSlot::FrontmatterFlattened,
    },
];
const HOOK_FIELDS: &[FieldMapping<RepoSlot>] = &[
    FieldMapping {
        key: "name",
        slot: RepoSlot::H1,
    },
    FieldMapping {
        key: "description",
        slot: RepoSlot::DescriptionParagraph,
    },
    FieldMapping {
        key: "instructions",
        slot: RepoSlot::Section("Instructions", SectionContent::BulletList),
    },
    FieldMapping {
        key: "inputs",
        slot: RepoSlot::FrontmatterKey("inputs"),
    },
    FieldMapping {
        key: "extensions",
        slot: RepoSlot::FrontmatterFlattened,
    },
];
const TEAM_FIELDS: &[FieldMapping<RepoSlot>] = &[
    FieldMapping {
        key: "name",
        slot: RepoSlot::H1,
    },
    FieldMapping {
        key: "description",
        slot: RepoSlot::DescriptionParagraph,
    },
    FieldMapping {
        key: "members",
        slot: RepoSlot::FrontmatterKey("members"),
    },
    FieldMapping {
        key: "include",
        slot: RepoSlot::FrontmatterKey("include"),
    },
    FieldMapping {
        key: "import",
        slot: RepoSlot::FrontmatterKey("import"),
    },
    FieldMapping {
        key: "extensions",
        slot: RepoSlot::FrontmatterFlattened,
    },
];
const ARTIFACT_KIND_FIELDS: &[FieldMapping<RepoSlot>] = &[
    FieldMapping {
        key: "name",
        slot: RepoSlot::H1,
    },
    FieldMapping {
        key: "description",
        slot: RepoSlot::DescriptionParagraph,
    },
    FieldMapping {
        key: "service",
        slot: RepoSlot::FrontmatterKey("service"),
    },
    FieldMapping {
        key: "access",
        slot: RepoSlot::FrontmatterKey("access"),
    },
    FieldMapping {
        key: "guidance",
        slot: RepoSlot::Section("Guidance", SectionContent::Paragraph),
    },
    FieldMapping {
        key: "extensions",
        slot: RepoSlot::FrontmatterFlattened,
    },
];
const WORKFLOW_FIELDS: &[FieldMapping<RepoSlot>] = &[
    FieldMapping {
        key: "name",
        slot: RepoSlot::H1,
    },
    FieldMapping {
        key: "description",
        slot: RepoSlot::DescriptionParagraph,
    },
    FieldMapping {
        key: "purpose",
        slot: RepoSlot::Section("Purpose", SectionContent::Paragraph),
    },
    FieldMapping {
        key: "raci",
        slot: RepoSlot::FrontmatterKey("raci"),
    },
    FieldMapping {
        key: "states",
        slot: RepoSlot::FrontmatterKey("states"),
    },
    FieldMapping {
        key: "steps",
        slot: RepoSlot::FrontmatterKey("steps"),
    },
    FieldMapping {
        key: "intercepts",
        slot: RepoSlot::FrontmatterKey("intercepts"),
    },
    FieldMapping {
        key: "guidance",
        slot: RepoSlot::Section("Guidance", SectionContent::Paragraph),
    },
    FieldMapping {
        key: "extensions",
        slot: RepoSlot::FrontmatterFlattened,
    },
];
const TASK_FIELDS: &[FieldMapping<RepoSlot>] = &[
    FieldMapping {
        key: "name",
        slot: RepoSlot::H1,
    },
    FieldMapping {
        key: "description",
        slot: RepoSlot::DescriptionParagraph,
    },
    FieldMapping {
        key: "purpose",
        slot: RepoSlot::Section("Purpose", SectionContent::Paragraph),
    },
    FieldMapping {
        key: "instructions",
        slot: RepoSlot::Section("Instructions", SectionContent::BulletList),
    },
    FieldMapping {
        key: "criteria",
        slot: RepoSlot::Section("Criteria", SectionContent::BulletList),
    },
    FieldMapping {
        key: "raci",
        slot: RepoSlot::FrontmatterKey("raci"),
    },
    FieldMapping {
        key: "artifact.kind",
        slot: RepoSlot::FrontmatterKey("artifact"),
    },
    FieldMapping {
        key: "states",
        slot: RepoSlot::FrontmatterKey("states"),
    },
    FieldMapping {
        key: "intercepts",
        slot: RepoSlot::FrontmatterKey("intercepts"),
    },
    FieldMapping {
        key: "guidance",
        slot: RepoSlot::Section("Guidance", SectionContent::Paragraph),
    },
    FieldMapping {
        key: "extensions",
        slot: RepoSlot::FrontmatterFlattened,
    },
];
const RELAY_FIELDS: &[FieldMapping<RepoSlot>] = &[
    FieldMapping {
        key: "name",
        slot: RepoSlot::H1,
    },
    FieldMapping {
        key: "description",
        slot: RepoSlot::DescriptionParagraph,
    },
    FieldMapping {
        key: "purpose",
        slot: RepoSlot::Section("Purpose", SectionContent::Paragraph),
    },
    FieldMapping {
        key: "raci",
        slot: RepoSlot::FrontmatterKey("raci"),
    },
    FieldMapping {
        key: "delegates_to",
        slot: RepoSlot::FrontmatterKey("delegates_to"),
    },
    FieldMapping {
        key: "briefing",
        slot: RepoSlot::Section("Briefing", SectionContent::Paragraph),
    },
    FieldMapping {
        key: "debriefing",
        slot: RepoSlot::Section("Debriefing", SectionContent::Paragraph),
    },
    FieldMapping {
        key: "state_map",
        slot: RepoSlot::FrontmatterKey("state_map"),
    },
    FieldMapping {
        key: "intercepts",
        slot: RepoSlot::FrontmatterKey("intercepts"),
    },
    FieldMapping {
        key: "guidance",
        slot: RepoSlot::Section("Guidance", SectionContent::Paragraph),
    },
    FieldMapping {
        key: "extensions",
        slot: RepoSlot::FrontmatterFlattened,
    },
];

macro_rules! simple_schema {
    ($path:expr, $fields:expr) => {
        EntitySchema::new(
            RefAssetDef {
                path_template: $path,
                kind: MARKDOWN_FILE,
                fields: $fields,
            },
            EMPTY_ASSETS,
        )
    };
}

impl SubstrateSchema<RepoSubstrate> for Role {
    const SCHEMA: EntitySchema<RepoSlot> = simple_schema!("common/roles/{id}.md", ROLE_FIELDS);
}
impl SubstrateSchema<RepoSubstrate> for Hook {
    const SCHEMA: EntitySchema<RepoSlot> = simple_schema!("common/hooks/{id}.md", HOOK_FIELDS);
}
impl SubstrateSchema<RepoSubstrate> for Team {
    const SCHEMA: EntitySchema<RepoSlot> = simple_schema!("common/teams/{id}.md", TEAM_FIELDS);
}
impl SubstrateSchema<RepoSubstrate> for ArtifactKind {
    const SCHEMA: EntitySchema<RepoSlot> =
        simple_schema!("common/artifact-kinds/{id}.md", ARTIFACT_KIND_FIELDS);
}
impl SubstrateSchema<RepoSubstrate> for Workflow {
    const SCHEMA: EntitySchema<RepoSlot> =
        simple_schema!("workflows/{id}/README.md", WORKFLOW_FIELDS);
}
impl SubstrateSchema<RepoSubstrate> for ReusableWorkflow {
    const SCHEMA: EntitySchema<RepoSlot> =
        simple_schema!("common/workflows/{id}/README.md", WORKFLOW_FIELDS);
}
impl SubstrateSchema<RepoSubstrate> for EmbeddedWorkflow {
    const SCHEMA: EntitySchema<RepoSlot> =
        simple_schema!("{parent.base}/{id}/README.md", WORKFLOW_FIELDS);
}
impl SubstrateSchema<RepoSubstrate> for Task {
    const SCHEMA: EntitySchema<RepoSlot> = EntitySchema::new(
        RefAssetDef {
            path_template: "{parent.base}/{id}/README.md",
            kind: MARKDOWN_FILE,
            fields: TASK_FIELDS,
        },
        &[AssetDef {
            path_template: "{parent.base}/{id}/template.md",
            kind: RAW_FILE,
            fields: &[FieldMapping {
                key: "artifact.template",
                slot: RepoSlot::FileContent,
            }],
            path_deps: &[],
        }],
    );
}
impl SubstrateSchema<RepoSubstrate> for Relay {
    const SCHEMA: EntitySchema<RepoSlot> =
        simple_schema!("{parent.base}/{id}/README.md", RELAY_FIELDS);
}
