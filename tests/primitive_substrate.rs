use pari::error::{primitive::*, ErrorLayer};

#[test]
fn document_primitive_default_error_type_is_snake_case() {
    let error = MalformedFrontmatter::new("frontmatter parse failed", "---".to_string());

    assert_eq!(error.error_layer(), ErrorLayer::Primitive);
    assert_eq!(error.error_type(), "malformed_frontmatter");
    assert_eq!(error.message(), "frontmatter parse failed");
}

#[test]
fn schema_primitive_captures_named_detail_fields() {
    let error = UnknownSchemaField::new("field is not mapped", "purpose".to_string());

    let details = error.details();
    assert_eq!(details.len(), 1);
    assert_eq!(details[0].field_name, "field");
    assert!(details[0].value.contains("purpose"));
}

#[test]
fn io_primitive_supports_multiple_detail_fields() {
    let error = PathPermissionDenied::new(
        "permission denied",
        "roles/eng-lead.md".to_string(),
        "write".to_string(),
    );

    let details = error.details();
    assert_eq!(details.len(), 2);
    assert_eq!(details[0].field_name, "asset_path");
    assert!(details[0].value.contains("roles/eng-lead.md"));
    assert_eq!(details[1].field_name, "operation");
    assert!(details[1].value.contains("write"));
}

#[test]
fn schema_message_only_primitive_has_no_details() {
    let error = SharedStateCorrupted::new("shared state corrupted");

    assert!(error.details().is_empty());
}

#[test]
fn payload_primitives_cover_shared_reconstruction_failures() {
    let error = MissingRequiredPayloadField::new("required payload field missing", "owner".to_string());

    assert_eq!(error.error_type(), "missing_required_payload_field");
    let details = error.details();
    assert_eq!(details.len(), 1);
    assert_eq!(details[0].field_name, "field");
    assert!(details[0].value.contains("owner"));
}
