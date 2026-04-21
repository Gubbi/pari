use super::super::schema::{AnyStructuralRule, ValidationSchema};
use super::structural::primitives::{
    kebab_case_id, non_empty_str, opt_non_empty_str, x_prefix_keys,
};
use crate::entity::entities::artifact_kind::{ArtifactKind, TrackedArtifactKind};

pub fn artifact_kind_validation_schema() -> ValidationSchema<ArtifactKind> {
    let mut structural: std::collections::HashMap<&'static str, Vec<AnyStructuralRule<ArtifactKind>>> =
        std::collections::HashMap::new();

    structural.insert(
        "entity_ref",
        vec![Box::new(|e: &TrackedArtifactKind| kebab_case_id(&e.entity_ref))],
    );
    structural.insert(
        "name",
        vec![Box::new(|e: &TrackedArtifactKind| {
            e.name.get().map(|v| non_empty_str(v)).unwrap_or_default()
        })],
    );
    structural.insert(
        "description",
        vec![Box::new(|e: &TrackedArtifactKind| {
            e.description.get().map(|v| opt_non_empty_str(v)).unwrap_or_default()
        })],
    );
    structural.insert(
        "service",
        vec![Box::new(|e: &TrackedArtifactKind| {
            e.service.get().map(|v| non_empty_str(v)).unwrap_or_default()
        })],
    );
    structural.insert(
        "access",
        vec![Box::new(|e: &TrackedArtifactKind| {
            e.access.get().map(|v| opt_non_empty_str(v)).unwrap_or_default()
        })],
    );
    structural.insert(
        "guidance",
        vec![Box::new(|e: &TrackedArtifactKind| {
            e.guidance.get().map(|v| opt_non_empty_str(v)).unwrap_or_default()
        })],
    );
    structural.insert(
        "extensions",
        vec![Box::new(|e: &TrackedArtifactKind| {
            e.extensions.get().map(|v| x_prefix_keys(v)).unwrap_or_default()
        })],
    );

    ValidationSchema {
        structural,
        semantic: std::collections::HashMap::new(),
        cross_entity: std::collections::HashMap::new(),
    }
}
