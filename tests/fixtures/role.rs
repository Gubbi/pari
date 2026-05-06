//! Canonical [`Role`] sample data for tests.
//!
//! Each function returns a fully-formed plain [`Role`] value with a
//! name that reads at the call site. Variants compose internally;
//! callers see only the named result. The workspace serializes the
//! plain entity at its insert boundary; tests do not construct a
//! tracked companion directly.

use pari::{entities::role::Role, entity::EntityRef};

/// Bare role with required fields populated.
///
/// `description` and `traits` are absent; `extensions` is empty.
pub fn a_minimal_role(id: &str) -> Role {
    role(id, "Minimal Role", None, None)
}

/// Role with both optional fields (`description`, `traits`) populated.
pub fn a_role_with_optional_fields(id: &str) -> Role {
    role(
        id,
        "Engineering Lead",
        Some("Owns delivery of the engineering roadmap."),
        Some(vec!["accountable", "technical"]),
    )
}

fn role(id: &str, name: &str, description: Option<&str>, traits: Option<Vec<&str>>) -> Role {
    Role {
        entity_ref: EntityRef::new(id),
        name: name.to_string(),
        description: description.map(str::to_string),
        purpose: "test purpose".to_string(),
        traits: traits.map(|ts| ts.into_iter().map(str::to_string).collect()),
        extensions: Default::default(),
    }
}
