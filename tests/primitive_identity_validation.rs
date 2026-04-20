use pari::error::{primitive::PrimitiveError, ErrorLayer};

#[test]
fn identity_primitive_defaults_error_type_from_name() {
    let error = PrimitiveError::ParentChildKindMismatch {
        context: PrimitiveError::context("parent kind does not match child kind"),
        parent_kind: "Workflow".to_string(),
        child_kind: "Task".to_string(),
    };

    assert_eq!(error.error_layer(), ErrorLayer::Primitive);
    assert_eq!(error.error_type(), "parent_child_kind_mismatch");
    assert_eq!(error.message(), "parent kind does not match child kind");
}

#[test]
fn identity_primitive_captures_reference_details() {
    let error = PrimitiveError::MissingRequiredReferenceField {
        context: PrimitiveError::context("missing required reference field"),
        field: "id".to_string(),
    };

    let details = error.details();
    assert_eq!(details.len(), 1);
    assert_eq!(details[0].field_name, "field");
    assert!(details[0].value.contains("id"));
}

#[test]
fn validation_primitive_captures_rule_violation_details() {
    let error = PrimitiveError::naming_format_violation(
        "structural rule rejected candidate state",
        Some("[0].id"),
        "camel_case",
    );

    let details = error.details();
    let sub_path_detail = details.iter().find(|d| d.field_name == "sub_path");
    let rule_kind_detail = details.iter().find(|d| d.field_name == "rule_kind");
    assert!(sub_path_detail.is_some(), "expected sub_path detail");
    assert!(sub_path_detail.unwrap().value.contains("[0].id"));
    assert!(rule_kind_detail.is_some(), "expected rule_kind detail");
    assert!(rule_kind_detail.unwrap().value.contains("camel_case"));
}

#[test]
fn validation_primitive_supports_dispatch_failures() {
    let error = PrimitiveError::ValidationDispatch {
        context: PrimitiveError::context("validation dispatch failed"),
        tracked_kind: "Workflow".to_string(),
        reason: "missing validator registration".to_string(),
    };

    assert_eq!(error.error_type(), "validation_dispatch");
    let details = error.details();
    assert_eq!(details.len(), 2);
    assert_eq!(details[0].field_name, "tracked_kind");
    assert_eq!(details[1].field_name, "reason");
}
