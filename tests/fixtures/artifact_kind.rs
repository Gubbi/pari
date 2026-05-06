//! Canonical [`ArtifactKind`] sample data for tests.

use pari::{entities::artifact_kind::ArtifactKind, entity::EntityRef};

/// Bare artifact kind with required fields populated.
pub fn a_minimal_artifact_kind(id: &str) -> ArtifactKind {
    ArtifactKind {
        entity_ref: EntityRef::new(id),
        name: "Design Doc".to_string(),
        description: None,
        service: "github".to_string(),
        access: None,
        guidance: None,
        extensions: Default::default(),
    }
}
