//! Payload and tracked-entity reconstruction primitive errors.

use pari_macros::primitive_with_fields;

/// A returned payload shape did not match the operation's expected response contract.
#[primitive_with_fields]
pub struct ResponseShapeMismatch {
    operation: String,
    expected: String,
    actual: String,
}

/// A tracked entity could not be projected into the payload shape required by a lower boundary.
#[primitive_with_fields]
pub struct EntityProjectionFailed {
    entity_ref: String,
    reason: String,
}

/// Encoded asset content could not be decoded into field values.
#[primitive_with_fields]
pub struct AssetDecodeFailed {
    asset_kind: String,
    field: String,
}

/// A decoded field map could not be merged back into tracked entity state.
#[primitive_with_fields]
pub struct FieldMapMergeFailed {
    field: String,
    reason: String,
}

/// A change payload could not be serialized into the required persistence boundary shape.
#[primitive_with_fields]
pub struct ChangePayloadSerializationFailed {
    entity_ref: String,
    change_kind: String,
}

/// A nested entity reference could not be serialized while building a payload.
#[primitive_with_fields]
pub struct InvalidNestedReferenceSerialization {
    field: String,
    entity_ref: String,
}

/// A decoded value did not match the type expected for the target field.
#[primitive_with_fields]
pub struct IncompatibleDecodedFieldType {
    field: String,
    expected_type: String,
    actual_type: String,
}

/// Dot-path reconstruction collided with an incompatible intermediate path segment.
#[primitive_with_fields]
pub struct DecodedPathSegmentCollision {
    path: String,
    segment: String,
}

/// A partial payload could not be deserialized into a coherent tracked entity shape.
#[primitive_with_fields]
pub struct PartialPayloadDeserializationFailed {
    entity_ref: String,
    reason: String,
}

/// Flattened extension keys conflicted with explicit payload keys.
#[primitive_with_fields]
pub struct ConflictingExtensionKeys {
    key: String,
}

/// A payload identified an entity kind different from the expected target kind.
#[primitive_with_fields]
pub struct EntityKindPayloadMismatch {
    entity_ref: String,
    expected_kind: String,
    actual_kind: String,
}

/// A payload omitted a field that is required for reconstruction.
#[primitive_with_fields]
pub struct MissingRequiredPayloadField {
    field: String,
}

/// A nested entity reference in an incoming payload failed its identity contract.
#[primitive_with_fields]
pub struct InvalidNestedEntityReference {
    field: String,
    entity_ref: String,
}

/// The overall payload shape could not be reconciled with the tracked entity definition.
#[primitive_with_fields]
pub struct IncompatibleTrackedEntityShape {
    entity_ref: String,
    reason: String,
}

/// A scalar value was required but a different payload shape was supplied.
#[primitive_with_fields]
pub struct ExpectedScalarValue {
    field: String,
    actual_type: String,
}

/// An object value was required but a different payload shape was supplied.
#[primitive_with_fields]
pub struct ExpectedObjectValue {
    field: String,
    actual_type: String,
}

/// An array value was required but a different payload shape was supplied.
#[primitive_with_fields]
pub struct ExpectedArrayValue {
    field: String,
    actual_type: String,
}

/// A field payload could not be encoded into JSON for an in-memory or external boundary.
#[primitive_with_fields]
pub struct JsonEncodingFailed {
    field: String,
    reason: String,
}

/// Extracted field data did not match the shape expected by the codec.
#[primitive_with_fields]
pub struct FieldExtractionShapeMismatch {
    field: String,
    expected_shape: String,
}

/// A raw JSON payload was malformed and could not be parsed safely.
#[primitive_with_fields]
pub struct MalformedJsonPayload {
    raw_snippet: String,
}

/// A decoded field map did not match the expected field-to-value mapping shape.
#[primitive_with_fields]
pub struct IncompatibleFieldMapShape {
    field: String,
    expected_shape: String,
}
