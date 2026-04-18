use pari::error::{primitive::*, ErrorLayer};

#[test]
fn identity_primitive_defaults_error_type_from_name() {
    let error = ParentChildKindMismatch::new(
        "parent kind does not match child kind",
        "Workflow".to_string(),
        "Task".to_string(),
    );

    assert_eq!(error.error_layer(), ErrorLayer::Primitive);
    assert_eq!(error.error_type(), "parent_child_kind_mismatch");
    assert_eq!(error.message(), "parent kind does not match child kind");
}

#[test]
fn identity_primitive_captures_reference_details() {
    let error = MissingRequiredReferenceField::new(
        "missing required reference field",
        "id".to_string(),
    );

    let details = error.details();
    assert_eq!(details.len(), 1);
    assert_eq!(details[0].field_name, "field");
    assert!(details[0].value.contains("id"));
}

#[test]
fn validation_primitive_captures_rule_violation_details() {
    let error = StructuralRuleViolation::new(
        "structural rule rejected candidate state",
        "steps.approval".to_string(),
        "non_empty".to_string(),
    );

    let details = error.details();
    assert_eq!(details.len(), 2);
    assert_eq!(details[0].field_name, "field_path");
    assert!(details[0].value.contains("steps.approval"));
    assert_eq!(details[1].field_name, "rule_kind");
    assert!(details[1].value.contains("non_empty"));
}

#[test]
fn validation_primitive_supports_dispatch_failures() {
    let error = ValidationDispatchFailed::new(
        "validation dispatch failed",
        "Workflow".to_string(),
        "missing validator registration".to_string(),
    );

    assert_eq!(error.error_type(), "validation_dispatch_failed");
    let details = error.details();
    assert_eq!(details.len(), 2);
    assert_eq!(details[0].field_name, "tracked_kind");
    assert_eq!(details[1].field_name, "reason");
}
