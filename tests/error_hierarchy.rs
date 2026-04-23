use pari::{
    error::{
        pari_error::PariError, primitive::PrimitiveError, ActivityError, ErrorCompose, FixDomain,
        OTelEmit, Recoverability,
    },
    substrate::{InMemorySubstrate, Substrate},
};

#[test]
fn activity_unpersistable_definition_is_data_operator_action() {
    let primitive = PrimitiveError::UnknownSchemaField {
        context: PrimitiveError::context("unknown schema field"),
        field: "name".to_string(),
    };
    let e = ActivityError::unpersistable_definition(InMemorySubstrate::substrate_name(), primitive);
    assert_eq!(e.fix_domain(), FixDomain::Data);
    assert_eq!(e.recoverability(), Recoverability::OperatorAction);
}

#[test]
fn activity_corrupt_persistence_state_is_infra_operator_action() {
    let primitive = PrimitiveError::PathPermissionDenied {
        context: PrimitiveError::context("path permission denied"),
        asset_path: "roles/eng-lead.md".to_string(),
        operation: "get".to_string(),
    };
    let e =
        ActivityError::corrupt_persistence_state(InMemorySubstrate::substrate_name(), primitive);
    assert_eq!(e.fix_domain(), FixDomain::Infra);
    assert_eq!(e.recoverability(), Recoverability::OperatorAction);
}

#[test]
fn activity_checkout_lifecycle_violation_is_client_user_action() {
    let primitive = PrimitiveError::already_checked_out("already checked out", "roles/eng-lead");
    let e = ActivityError::checkout_lifecycle_violation("store", primitive);
    assert_eq!(e.fix_domain(), FixDomain::Client);
    assert_eq!(e.recoverability(), Recoverability::UserAction);
}

#[test]
fn activity_non_existent_data_is_client_user_action() {
    let primitive = PrimitiveError::entity_not_found("entity not found", "roles/ghost");
    let e = ActivityError::non_existent_data("store", primitive);
    assert_eq!(e.fix_domain(), FixDomain::Client);
    assert_eq!(e.recoverability(), Recoverability::UserAction);
}

#[test]
fn activity_workspace_not_clean_is_client_user_action() {
    let primitive = PrimitiveError::pending_checkouts("pending checkouts", 1);
    let e = ActivityError::workspace_not_clean("store", primitive);
    assert_eq!(e.fix_domain(), FixDomain::Client);
    assert_eq!(e.recoverability(), Recoverability::UserAction);
}

#[test]
fn activity_store_unavailable_is_pari_not_recoverable() {
    let primitive = PrimitiveError::store_unavailable("channel closed");
    let e = ActivityError::store_unavailable("server", primitive);
    assert_eq!(e.fix_domain(), FixDomain::Pari);
    assert_eq!(e.recoverability(), Recoverability::NotRecoverable);
}

#[test]
fn activity_validation_failed_is_client_user_action() {
    let primitive = PrimitiveError::field_validation_error(
        "validation failed",
        std::collections::HashMap::new(),
    );
    let e = ActivityError::validation_failed("validation.runner", primitive);
    assert_eq!(e.fix_domain(), FixDomain::Client);
    assert_eq!(e.recoverability(), Recoverability::UserAction);
}

#[test]
fn pari_error_wraps_activity_error() {
    let primitive = PrimitiveError::UnknownSchemaField {
        context: PrimitiveError::context("unknown schema field"),
        field: "name".to_string(),
    };
    let activity =
        ActivityError::unpersistable_definition(InMemorySubstrate::substrate_name(), primitive);
    let pari = PariError::LoadFailed(activity);
    assert_eq!(pari.fix_domain(), FixDomain::Data);
}

#[test]
fn emit_on_pari_error_does_not_panic() {
    let primitive = PrimitiveError::pending_checkouts("pending checkouts", 1);
    let activity = ActivityError::workspace_not_clean("store", primitive);
    let e = PariError::SaveFailed(activity);
    e.emit();
}

fn assert_error_compose<E: ErrorCompose>() {}

#[test]
fn all_error_types_implement_error_compose() {
    assert_error_compose::<ActivityError>();
    assert_error_compose::<PariError>();
}
