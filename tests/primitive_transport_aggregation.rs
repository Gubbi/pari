use pari::error::{primitive::*, ErrorLayer};

#[test]
fn transport_primitive_defaults_error_type_from_name() {
    let error = RequestChannelSendFailed::new(
        "request channel send failed",
        "resolve".to_string(),
        "workspace->store".to_string(),
    );

    assert_eq!(error.error_layer(), ErrorLayer::Primitive);
    assert_eq!(error.error_type(), "request_channel_send_failed");
    assert_eq!(error.message(), "request channel send failed");
}

#[test]
fn transport_primitive_captures_boundary_details() {
    let error = ReplyChannelDropped::new(
        "reply channel dropped",
        "persist".to_string(),
        "store->workspace".to_string(),
    );

    let details = error.details();
    assert_eq!(details.len(), 2);
    assert_eq!(details[0].field_name, "operation");
    assert!(details[0].value.contains("persist"));
    assert_eq!(details[1].field_name, "boundary");
    assert!(details[1].value.contains("store->workspace"));
}

#[test]
fn aggregation_primitive_captures_batch_shape_details() {
    let error = HeterogeneousBatch::new(
        "batch contained incompatible operation contexts",
        "substrate_errors".to_string(),
        "mixed load and persist failures".to_string(),
    );

    assert_eq!(error.error_type(), "heterogeneous_batch");
    let details = error.details();
    assert_eq!(details.len(), 2);
    assert_eq!(details[0].field_name, "batch_kind");
    assert_eq!(details[1].field_name, "conflict");
}
