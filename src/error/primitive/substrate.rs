//! Substrate-specific primitive errors that do not fit a broader shared family.

use pari_macros::primitive_with_fields;

/// The schema registry has no entry for the requested entity kind.
#[primitive_with_fields]
pub struct UnsupportedEntityKind {
    entity_kind: String,
}
