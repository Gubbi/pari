use pari::error::{FixDomain, Recoverability, Severity, ErrorCompose};
use pari::substrate::pipeline::{codec::error::CodecError, executor::error::ExecutorError};
use pari::substrate::error::SubstrateError;
use pari::store_error::StoreError;
use pari::workspace::{CheckoutError, CommitError, LoadError, PersistError, ResolveError, UndoError};
use pari::validation::error::{SetterError, ValidationErrors, FieldValidationError, ValidationKind};
use pari::error::pari_error::PariError;

// --- Primitive classifications ---

#[test]
fn codec_error_is_data_operator_action() {
    let e = CodecError::new("name", "expected string");
    assert_eq!(e.fix_domain(),     FixDomain::Data);
    assert_eq!(e.recoverability(), Recoverability::OperatorAction);
}

#[test]
fn executor_error_is_infra_operator_action() {
    let e = ExecutorError::new("roles/eng-lead.md", "permission denied");
    assert_eq!(e.fix_domain(),     FixDomain::Infra);
    assert_eq!(e.recoverability(), Recoverability::OperatorAction);
}

// --- SubstrateError delegates correctly ---

#[test]
fn substrate_error_codec_variant_delegates_fix_domain() {
    let sub = SubstrateError::Codec(CodecError::new("name", "bad"));
    assert_eq!(sub.fix_domain(), FixDomain::Data);
}

#[test]
fn substrate_error_executor_variant_delegates_fix_domain() {
    let sub = SubstrateError::Executor(ExecutorError::new("roles/x.md", "io error"));
    assert_eq!(sub.fix_domain(), FixDomain::Infra);
}

// --- Store operation error classifications ---

#[test]
fn checkout_already_checked_out_is_client_user_action() {
    let e = CheckoutError::AlreadyCheckedOut { entity_ref: "roles/eng-lead".into() };
    assert_eq!(e.fix_domain(),     FixDomain::Client);
    assert_eq!(e.recoverability(), Recoverability::UserAction);
    assert_eq!(e.severity(),       Severity::Warn);
}

#[test]
fn checkout_not_found_is_client_user_action() {
    let e = CheckoutError::EntityNotFound { entity_ref: "roles/eng-lead".into() };
    assert_eq!(e.fix_domain(),     FixDomain::Client);
    assert_eq!(e.recoverability(), Recoverability::UserAction);
}

#[test]
fn checkout_substrate_delegates() {
    let e = CheckoutError::Substrate(SubstrateError::Executor(
        ExecutorError::new("roles/x.md", "io error")
    ));
    assert_eq!(e.fix_domain(), FixDomain::Infra);
}

#[test]
fn commit_validation_failed_is_client_user_action() {
    let e = CommitError::ValidationFailed {
        error_count: 1,
        errors: ValidationErrors::new(),
    };
    assert_eq!(e.fix_domain(),     FixDomain::Client);
    assert_eq!(e.recoverability(), Recoverability::UserAction);
}

#[test]
fn commit_cross_ref_check_failed_delegates_to_substrate() {
    let e = CommitError::CrossReferenceCheckFailed(
        SubstrateError::Executor(ExecutorError::new("roles/x.md", "io"))
    );
    assert_eq!(e.fix_domain(), FixDomain::Infra);
}

#[test]
fn commit_store_unavailable_is_pari_not_recoverable() {
    let e = CommitError::StoreUnavailable(StoreError::Unavailable);
    assert_eq!(e.fix_domain(),     FixDomain::Pari);
    assert_eq!(e.recoverability(), Recoverability::NotRecoverable);
}

#[test]
fn load_validation_failed_is_data_operator_action() {
    let e = LoadError::ValidationFailed {
        error_count: 2,
        errors: ValidationErrors::new(),
    };
    // Data because substrate returned invalid content, not user input
    assert_eq!(e.fix_domain(),     FixDomain::Data);
    assert_eq!(e.recoverability(), Recoverability::OperatorAction);
}

#[test]
fn persist_pending_checkouts_is_client_user_action() {
    let e = PersistError::PendingCheckouts { checked_out_count: 3 };
    assert_eq!(e.fix_domain(),     FixDomain::Client);
    assert_eq!(e.recoverability(), Recoverability::UserAction);
}

#[test]
fn undo_wrong_state_is_pari_not_recoverable() {
    let e = UndoError::WrongState;
    assert_eq!(e.fix_domain(),     FixDomain::Pari);
    assert_eq!(e.recoverability(), Recoverability::NotRecoverable);
}

#[test]
fn resolve_store_unavailable_is_pari_not_recoverable() {
    let e = ResolveError::StoreUnavailable(StoreError::Unavailable);
    assert_eq!(e.fix_domain(),     FixDomain::Pari);
    assert_eq!(e.recoverability(), Recoverability::NotRecoverable);
}

// --- as_error downcast through PariError chain ---

#[test]
fn pari_error_downcast_reaches_load_error() {
    let codec = CodecError::new("name", "bad");
    let sub   = SubstrateError::Codec(codec);
    let load  = LoadError::Substrate(sub);
    let pari  = PariError::LoadFailed(load);

    let found = (&pari as &dyn ErrorCompose).as_error::<LoadError>();
    assert!(found.is_some());
}

// --- emit() compiles and is callable ---

#[test]
fn emit_on_pari_error_does_not_panic() {
    use pari::error::OTelEmit;
    let e = PariError::SaveFailed(PersistError::PendingCheckouts { checked_out_count: 1 });
    e.emit();
}

// --- ValidationErrors: plain data, not ErrorCompose ---

#[test]
fn validation_errors_accumulate() {
    let mut errs = ValidationErrors::new();
    errs.errors.push(FieldValidationError {
        path:    "id".into(),
        message: "must be kebab-case".into(),
        kind:    ValidationKind::Structural,
    });
    assert_eq!(errs.errors.len(), 1);
}

// --- Setter error ---

#[test]
fn setter_validation_error_is_client_user_action() {
    let e = SetterError::Validation {
        error_count: 1,
        errors: ValidationErrors::new(),
    };
    assert_eq!(e.fix_domain(),     FixDomain::Client);
    assert_eq!(e.recoverability(), Recoverability::UserAction);
}

// --- All operation errors implement ErrorCompose ---

fn assert_error_compose<E: ErrorCompose>() {}

#[test]
fn all_operation_errors_implement_error_compose() {
    assert_error_compose::<CodecError>();
    assert_error_compose::<ExecutorError>();
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
