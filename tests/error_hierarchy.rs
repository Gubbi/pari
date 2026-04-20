use pari::{
    error::{
        pari_error::PariError, primitive::PrimitiveError, ErrorCompose, FixDomain, Recoverability,
        Severity,
    },
    store_error::StoreError,
    substrate::error::SubstrateError,
    validation::error::{FieldValidationError, SetterError, ValidationErrors, ValidationKind},
    workspace::{CheckoutError, CommitError, LoadError, PersistError, ResolveError, UndoError},
};

#[test]
fn substrate_unpersistable_definition_is_data_operator_action() {
    let primitive = PrimitiveError::UnknownSchemaField {
        context: PrimitiveError::context("unknown schema field"),
        field: "name".to_string(),
    };
    let e = SubstrateError::unpersistable_definition(primitive);
    assert_eq!(e.fix_domain(), FixDomain::Data);
    assert_eq!(e.recoverability(), Recoverability::OperatorAction);
}

#[test]
fn substrate_corrupt_persistence_state_is_infra_operator_action() {
    let primitive = PrimitiveError::PathPermissionDenied {
        context: PrimitiveError::context("path permission denied"),
        asset_path: "roles/eng-lead.md".to_string(),
        operation: "get".to_string(),
    };
    let e = SubstrateError::corrupt_persistence_state(primitive);
    assert_eq!(e.fix_domain(), FixDomain::Infra);
    assert_eq!(e.recoverability(), Recoverability::OperatorAction);
}

#[test]
fn checkout_already_checked_out_is_client_user_action() {
    let e = CheckoutError::AlreadyCheckedOut {
        entity_ref: "roles/eng-lead".into(),
    };
    assert_eq!(e.fix_domain(), FixDomain::Client);
    assert_eq!(e.recoverability(), Recoverability::UserAction);
    assert_eq!(e.severity(), Severity::Warn);
}

#[test]
fn checkout_not_found_is_client_user_action() {
    let e = CheckoutError::EntityNotFound {
        entity_ref: "roles/eng-lead".into(),
    };
    assert_eq!(e.fix_domain(), FixDomain::Client);
    assert_eq!(e.recoverability(), Recoverability::UserAction);
}

#[test]
fn checkout_substrate_delegates() {
    let primitive = PrimitiveError::FileRead {
        context: PrimitiveError::context("file read failed"),
        asset_path: "roles/x.md".to_string(),
    };
    let e = CheckoutError::Substrate(SubstrateError::corrupt_persistence_state(primitive));
    assert_eq!(e.fix_domain(), FixDomain::Infra);
}

#[test]
fn commit_validation_failed_is_client_user_action() {
    let e = CommitError::ValidationFailed {
        error_count: 1,
        errors: ValidationErrors::new(),
    };
    assert_eq!(e.fix_domain(), FixDomain::Client);
    assert_eq!(e.recoverability(), Recoverability::UserAction);
}

#[test]
fn commit_cross_ref_check_failed_delegates_to_substrate() {
    let primitive = PrimitiveError::FileRead {
        context: PrimitiveError::context("file read failed"),
        asset_path: "roles/x.md".to_string(),
    };
    let e = CommitError::CrossReferenceCheckFailed(SubstrateError::corrupt_persistence_state(
        primitive,
    ));
    assert_eq!(e.fix_domain(), FixDomain::Infra);
}

#[test]
fn commit_store_unavailable_is_pari_not_recoverable() {
    let e = CommitError::StoreUnavailable(StoreError::Unavailable);
    assert_eq!(e.fix_domain(), FixDomain::Pari);
    assert_eq!(e.recoverability(), Recoverability::NotRecoverable);
}

#[test]
fn load_validation_failed_is_data_operator_action() {
    let e = LoadError::ValidationFailed {
        error_count: 2,
        errors: ValidationErrors::new(),
    };
    assert_eq!(e.fix_domain(), FixDomain::Data);
    assert_eq!(e.recoverability(), Recoverability::OperatorAction);
}

#[test]
fn persist_pending_checkouts_is_client_user_action() {
    let e = PersistError::PendingCheckouts {
        checked_out_count: 3,
    };
    assert_eq!(e.fix_domain(), FixDomain::Client);
    assert_eq!(e.recoverability(), Recoverability::UserAction);
}

#[test]
fn undo_wrong_state_is_pari_not_recoverable() {
    let e = UndoError::WrongState;
    assert_eq!(e.fix_domain(), FixDomain::Pari);
    assert_eq!(e.recoverability(), Recoverability::NotRecoverable);
}

#[test]
fn resolve_store_unavailable_is_pari_not_recoverable() {
    let e = ResolveError::StoreUnavailable(StoreError::Unavailable);
    assert_eq!(e.fix_domain(), FixDomain::Pari);
    assert_eq!(e.recoverability(), Recoverability::NotRecoverable);
}

#[test]
fn pari_error_downcast_reaches_load_error() {
    let primitive = PrimitiveError::UnknownSchemaField {
        context: PrimitiveError::context("unknown schema field"),
        field: "name".to_string(),
    };
    let sub = SubstrateError::unpersistable_definition(primitive);
    let load = LoadError::Substrate(sub);
    let pari = PariError::LoadFailed(load);

    let found = (&pari as &dyn ErrorCompose).as_error::<LoadError>();
    assert!(found.is_some());
}

#[test]
fn emit_on_pari_error_does_not_panic() {
    use pari::error::OTelEmit;
    let e = PariError::SaveFailed(PersistError::PendingCheckouts {
        checked_out_count: 1,
    });
    e.emit();
}

#[test]
fn validation_errors_accumulate() {
    let mut errs = ValidationErrors::new();
    errs.errors.push(FieldValidationError {
        path: "id".into(),
        message: "must be kebab-case".into(),
        kind: ValidationKind::Structural,
    });
    assert_eq!(errs.errors.len(), 1);
}

#[test]
fn setter_validation_error_is_client_user_action() {
    let e = SetterError::Validation {
        error_count: 1,
        errors: ValidationErrors::new(),
    };
    assert_eq!(e.fix_domain(), FixDomain::Client);
    assert_eq!(e.recoverability(), Recoverability::UserAction);
}

fn assert_error_compose<E: ErrorCompose>() {}

#[test]
fn all_operation_errors_implement_error_compose() {
    assert_error_compose::<SubstrateError>();
    assert_error_compose::<StoreError>();
    assert_error_compose::<CheckoutError>();
    assert_error_compose::<CommitError>();
    assert_error_compose::<LoadError>();
    assert_error_compose::<UndoError>();
    assert_error_compose::<PersistError>();
    assert_error_compose::<ResolveError>();
    assert_error_compose::<SetterError>();
    assert_error_compose::<PariError>();
}
