//! Entity identity and reference primitive errors.

use pari_macros::primitive_with_fields;

/// A top-level entity identifier did not satisfy the required identifier format.
#[primitive_with_fields]
pub struct InvalidTopLevelIdentifierFormat {
    id: String,
}

/// An identifier was syntactically valid but belongs to a reserved identity space.
#[primitive_with_fields]
pub struct ReservedIdentifierValue {
    id: String,
}

/// An identifier could not be normalized into the canonical stored form.
#[primitive_with_fields]
pub struct IdentifierCanonicalizationFailed {
    id: String,
    reason: String,
}

/// An embedded child identifier did not satisfy the required identifier format.
#[primitive_with_fields]
pub struct InvalidEmbeddedIdentifierFormat {
    id: String,
    child_kind: String,
}

/// A parent identity was incompatible with the requested child entity kind.
#[primitive_with_fields]
pub struct ParentChildKindMismatch {
    parent_kind: String,
    child_kind: String,
}

/// Required parent identity data was missing from a child reference.
#[primitive_with_fields]
pub struct MissingParentIdentityComponent {
    child_kind: String,
    component: String,
}

/// A parent chain described an impossible or cyclic entity hierarchy.
#[primitive_with_fields]
pub struct ImpossibleParentChain {
    child_kind: String,
    parent_path: String,
}

/// A serialized reference omitted one or more fields required to reconstruct identity.
#[primitive_with_fields]
pub struct MissingRequiredReferenceField {
    field: String,
}

/// A serialized reference contained an entity kind tag that is not recognized.
#[primitive_with_fields]
pub struct UnknownEntityKindTag {
    entity_kind: String,
}

/// Parent identity data was present but could not be parsed into a valid parent reference.
#[primitive_with_fields]
pub struct MalformedParentPayload {
    parent_kind: String,
    reason: String,
}

/// A reference payload combined a valid child kind with an incompatible parent kind.
#[primitive_with_fields]
pub struct ReferenceParentKindMismatch {
    parent_kind: String,
    child_kind: String,
}

/// A serialized identifier field existed but used the wrong scalar or structured representation.
#[primitive_with_fields]
pub struct IdentifierPayloadTypeMismatch {
    field: String,
    actual_type: String,
}

/// A serialized reference payload contained overlapping or contradictory identity data.
#[primitive_with_fields]
pub struct ConflictingReferenceFields {
    field: String,
    conflict: String,
}

/// Parent identity was mandatory but the required parent object was missing.
#[primitive_with_fields]
pub struct MissingRequiredParentObject {
    child_kind: String,
}

/// A top-level entity payload incorrectly included parent identity data.
#[primitive_with_fields]
pub struct UnexpectedParentOnTopLevelEntity {
    entity_kind: String,
}

/// A nested parent reference was present but structurally invalid.
#[primitive_with_fields]
pub struct MalformedNestedParentReference {
    parent_kind: String,
    reason: String,
}
