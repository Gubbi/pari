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
            AssetKind, EntitySchema, FieldMapping, RefAssetDef, SubstrateSchema, ValueSlot,
        },
        InMemorySubstrate,
    },
};

const STRUCTURED_ASSET: &AssetKind = &AssetKind {
    distinguishes_create: false,
    supports_partial: false,
};
const ROLE_FIELDS: &[FieldMapping<ValueSlot>] = &[
    FieldMapping {
        key: "name",
        slot: ValueSlot::Value,
    },
    FieldMapping {
        key: "description",
        slot: ValueSlot::Value,
    },
    FieldMapping {
        key: "purpose",
        slot: ValueSlot::Value,
    },
    FieldMapping {
        key: "traits",
        slot: ValueSlot::Value,
    },
    FieldMapping {
        key: "extensions",
        slot: ValueSlot::Value,
    },
];
const HOOK_FIELDS: &[FieldMapping<ValueSlot>] = &[
    FieldMapping {
        key: "name",
        slot: ValueSlot::Value,
    },
    FieldMapping {
        key: "description",
        slot: ValueSlot::Value,
    },
    FieldMapping {
        key: "instructions",
        slot: ValueSlot::Value,
    },
    FieldMapping {
        key: "inputs",
        slot: ValueSlot::Value,
    },
    FieldMapping {
        key: "extensions",
        slot: ValueSlot::Value,
    },
];
const TEAM_FIELDS: &[FieldMapping<ValueSlot>] = &[
    FieldMapping {
        key: "name",
        slot: ValueSlot::Value,
    },
    FieldMapping {
        key: "description",
        slot: ValueSlot::Value,
    },
    FieldMapping {
        key: "members",
        slot: ValueSlot::Value,
    },
    FieldMapping {
        key: "include",
        slot: ValueSlot::Value,
    },
    FieldMapping {
        key: "import",
        slot: ValueSlot::Value,
    },
    FieldMapping {
        key: "extensions",
        slot: ValueSlot::Value,
    },
];
const ARTIFACT_KIND_FIELDS: &[FieldMapping<ValueSlot>] = &[
    FieldMapping {
        key: "name",
        slot: ValueSlot::Value,
    },
    FieldMapping {
        key: "description",
        slot: ValueSlot::Value,
    },
    FieldMapping {
        key: "service",
        slot: ValueSlot::Value,
    },
    FieldMapping {
        key: "access",
        slot: ValueSlot::Value,
    },
    FieldMapping {
        key: "guidance",
        slot: ValueSlot::Value,
    },
    FieldMapping {
        key: "extensions",
        slot: ValueSlot::Value,
    },
];
const WORKFLOW_FIELDS: &[FieldMapping<ValueSlot>] = &[
    FieldMapping {
        key: "name",
        slot: ValueSlot::Value,
    },
    FieldMapping {
        key: "description",
        slot: ValueSlot::Value,
    },
    FieldMapping {
        key: "purpose",
        slot: ValueSlot::Value,
    },
    FieldMapping {
        key: "raci",
        slot: ValueSlot::Value,
    },
    FieldMapping {
        key: "states",
        slot: ValueSlot::Value,
    },
    FieldMapping {
        key: "steps",
        slot: ValueSlot::Value,
    },
    FieldMapping {
        key: "intercepts",
        slot: ValueSlot::Value,
    },
    FieldMapping {
        key: "guidance",
        slot: ValueSlot::Value,
    },
    FieldMapping {
        key: "extensions",
        slot: ValueSlot::Value,
    },
];
const TASK_FIELDS: &[FieldMapping<ValueSlot>] = &[
    FieldMapping {
        key: "name",
        slot: ValueSlot::Value,
    },
    FieldMapping {
        key: "description",
        slot: ValueSlot::Value,
    },
    FieldMapping {
        key: "purpose",
        slot: ValueSlot::Value,
    },
    FieldMapping {
        key: "instructions",
        slot: ValueSlot::Value,
    },
    FieldMapping {
        key: "criteria",
        slot: ValueSlot::Value,
    },
    FieldMapping {
        key: "raci",
        slot: ValueSlot::Value,
    },
    FieldMapping {
        key: "artifact.kind",
        slot: ValueSlot::Value,
    },
    FieldMapping {
        key: "artifact.template",
        slot: ValueSlot::Value,
    },
    FieldMapping {
        key: "states",
        slot: ValueSlot::Value,
    },
    FieldMapping {
        key: "intercepts",
        slot: ValueSlot::Value,
    },
    FieldMapping {
        key: "guidance",
        slot: ValueSlot::Value,
    },
    FieldMapping {
        key: "extensions",
        slot: ValueSlot::Value,
    },
];
const RELAY_FIELDS: &[FieldMapping<ValueSlot>] = &[
    FieldMapping {
        key: "name",
        slot: ValueSlot::Value,
    },
    FieldMapping {
        key: "description",
        slot: ValueSlot::Value,
    },
    FieldMapping {
        key: "purpose",
        slot: ValueSlot::Value,
    },
    FieldMapping {
        key: "raci",
        slot: ValueSlot::Value,
    },
    FieldMapping {
        key: "delegates_to",
        slot: ValueSlot::Value,
    },
    FieldMapping {
        key: "briefing",
        slot: ValueSlot::Value,
    },
    FieldMapping {
        key: "debriefing",
        slot: ValueSlot::Value,
    },
    FieldMapping {
        key: "state_map",
        slot: ValueSlot::Value,
    },
    FieldMapping {
        key: "intercepts",
        slot: ValueSlot::Value,
    },
    FieldMapping {
        key: "guidance",
        slot: ValueSlot::Value,
    },
    FieldMapping {
        key: "extensions",
        slot: ValueSlot::Value,
    },
];

macro_rules! simple_schema {
    ($path:expr, $fields:expr) => {
        EntitySchema::new(
            RefAssetDef {
                path_template: $path,
                kind: STRUCTURED_ASSET,
                fields: $fields,
            },
            &[],
        )
    };
}

impl SubstrateSchema<InMemorySubstrate> for Role {
    const SCHEMA: EntitySchema<ValueSlot> = simple_schema!("roles/{id}", ROLE_FIELDS);
}
impl SubstrateSchema<InMemorySubstrate> for Hook {
    const SCHEMA: EntitySchema<ValueSlot> = simple_schema!("hooks/{id}", HOOK_FIELDS);
}
impl SubstrateSchema<InMemorySubstrate> for Team {
    const SCHEMA: EntitySchema<ValueSlot> = simple_schema!("teams/{id}", TEAM_FIELDS);
}
impl SubstrateSchema<InMemorySubstrate> for ArtifactKind {
    const SCHEMA: EntitySchema<ValueSlot> =
        simple_schema!("artifact-kinds/{id}", ARTIFACT_KIND_FIELDS);
}
impl SubstrateSchema<InMemorySubstrate> for Workflow {
    const SCHEMA: EntitySchema<ValueSlot> = simple_schema!("workflows/{id}", WORKFLOW_FIELDS);
}
impl SubstrateSchema<InMemorySubstrate> for ReusableWorkflow {
    const SCHEMA: EntitySchema<ValueSlot> =
        simple_schema!("reusable-workflows/{id}", WORKFLOW_FIELDS);
}
impl SubstrateSchema<InMemorySubstrate> for EmbeddedWorkflow {
    const SCHEMA: EntitySchema<ValueSlot> = simple_schema!("{parent.base}/{id}", WORKFLOW_FIELDS);
}
impl SubstrateSchema<InMemorySubstrate> for Task {
    const SCHEMA: EntitySchema<ValueSlot> = EntitySchema::new(
        RefAssetDef {
            path_template: "{parent.base}/{id}",
            kind: STRUCTURED_ASSET,
            fields: TASK_FIELDS,
        },
        &[],
    );
}
impl SubstrateSchema<InMemorySubstrate> for Relay {
    const SCHEMA: EntitySchema<ValueSlot> = simple_schema!("{parent.base}/{id}", RELAY_FIELDS);
}
