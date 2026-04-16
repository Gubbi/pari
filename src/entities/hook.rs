use crate::{
    entity::{EntityKind, EntityRef},
    types::Extensions,
};

#[derive(pari_macros::Entity)]
#[entity(kind = EntityKind::Hook, schema = crate::validation::hook::hook_validation_schema)]
pub struct Hook {
    pub entity_ref: EntityRef<Hook>,
    pub name: String,
    pub description: Option<String>,
    pub instructions: Vec<String>,
    pub inputs: Option<Vec<HookInput>>,
    pub extensions: Extensions,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct HookInput {
    pub name: String,
    pub description: Option<String>,
    pub required: bool,
}
