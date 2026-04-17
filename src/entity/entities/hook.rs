use crate::entity::{types::Extensions, EntityKind, EntityRef};

#[derive(
    Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, pari_macros::Entity,
)]
#[schemars(deny_unknown_fields)]
#[entity(kind = EntityKind::Hook, schema = crate::validation::hook::hook_validation_schema)]
pub struct Hook {
    pub entity_ref: EntityRef<Hook>,
    pub name: String,
    pub description: Option<String>,
    #[schemars(length(min = 1))]
    pub instructions: Vec<String>,
    pub inputs: Option<Vec<HookInput>>,
    #[serde(flatten)]
    pub extensions: Extensions,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(deny_unknown_fields)]
pub struct HookInput {
    pub name: String,
    pub description: Option<String>,
    pub required: bool,
}
