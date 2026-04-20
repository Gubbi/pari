use pari::error::{primitive::PrimitiveError, ErrorLayer};

#[test]
fn document_primitive_default_error_type_is_snake_case() {
    let error = PrimitiveError::MalformedFrontmatter {
        context: PrimitiveError::context("frontmatter parse failed"),
        raw_snippet: "---".to_string(),
    };

    assert_eq!(error.error_layer(), ErrorLayer::Primitive);
    assert_eq!(error.error_type(), "malformed_frontmatter");
    assert_eq!(error.message(), "frontmatter parse failed");
}

#[test]
fn schema_primitive_captures_named_detail_fields() {
    let error = PrimitiveError::UnknownSchemaField {
        context: PrimitiveError::context("field is not mapped"),
        field: "purpose".to_string(),
    };

    let details = error.details();
    assert_eq!(details.len(), 1);
    assert_eq!(details[0].field_name, "field");
    assert!(details[0].value.contains("purpose"));
}

#[test]
fn io_primitive_supports_multiple_detail_fields() {
    let error = PrimitiveError::PathPermissionDenied {
        context: PrimitiveError::context("permission denied"),
        asset_path: "roles/eng-lead.md".to_string(),
        operation: "write".to_string(),
    };

    let details = error.details();
    assert_eq!(details.len(), 2);
    assert_eq!(details[0].field_name, "asset_path");
    assert!(details[0].value.contains("roles/eng-lead.md"));
    assert_eq!(details[1].field_name, "operation");
    assert!(details[1].value.contains("write"));
}

#[test]
fn schema_message_only_primitive_has_no_details() {
    let error = PrimitiveError::SharedStateCorrupted {
        context: PrimitiveError::context("shared state corrupted"),
    };

    assert!(error.details().is_empty());
}

#[test]
fn payload_primitives_cover_shared_reconstruction_failures() {
    let error = PrimitiveError::MissingRequiredPayloadField {
        context: PrimitiveError::context("required payload field missing"),
        field: "owner".to_string(),
    };

    assert_eq!(error.error_type(), "missing_required_payload_field");
    let details = error.details();
    assert_eq!(details.len(), 1);
    assert_eq!(details[0].field_name, "field");
    assert!(details[0].value.contains("owner"));
}
