use pari::error::{FixDomain, Recoverability, Severity, ErrorCompose, OTelEmit, BatchError};
use pari_macros::{ErrorCompose, OTelEmit};
use thiserror::Error;

// --- Classification types ---

#[test]
fn severity_pari_not_recoverable_is_error() {
    assert_eq!(
        Severity::from_classification(FixDomain::Pari, Recoverability::NotRecoverable),
        Severity::Error
    );
}

#[test]
fn severity_client_user_action_is_warn() {
    assert_eq!(
        Severity::from_classification(FixDomain::Client, Recoverability::UserAction),
        Severity::Warn
    );
}

#[test]
fn severity_infra_retryable_is_warn() {
    assert_eq!(
        Severity::from_classification(FixDomain::Infra, Recoverability::Retryable),
        Severity::Warn
    );
}

#[test]
fn severity_data_operator_action_is_error() {
    assert_eq!(
        Severity::from_classification(FixDomain::Data, Recoverability::OperatorAction),
        Severity::Error
    );
}

// --- ErrorCompose derive: Activity (declaring) ---

#[derive(Error, Debug, ErrorCompose)]
#[error("data error: {message}")]
#[compose(fix = Data, recoverability = OperatorAction)]
struct TestDataError {
    message: String,
    hint: Option<String>,
}

#[test]
fn activity_fix_domain() {
    let e = TestDataError { message: "bad".into(), hint: None };
    assert_eq!(e.fix_domain(), FixDomain::Data);
}

#[test]
fn activity_recoverability() {
    let e = TestDataError { message: "bad".into(), hint: None };
    assert_eq!(e.recoverability(), Recoverability::OperatorAction);
}

#[test]
fn activity_severity_derived() {
    let e = TestDataError { message: "bad".into(), hint: None };
    assert_eq!(e.severity(), Severity::Error);
}

// --- ErrorCompose derive: Intermediary Op (delegating enum) ---

#[derive(Error, Debug, ErrorCompose)]
#[error("client error")]
#[compose(fix = Client, recoverability = UserAction)]
struct TestClientError;

#[derive(Error, Debug, ErrorCompose)]
pub enum TestOpError {
    #[error(transparent)]
    #[compose(delegate)]
    Data(TestDataError),

    #[error(transparent)]
    #[compose(delegate)]
    Client(TestClientError),
}

#[test]
fn delegating_propagates_data_fix_domain() {
    let e = TestOpError::Data(TestDataError { message: "x".into(), hint: None });
    assert_eq!(e.fix_domain(), FixDomain::Data);
}

#[test]
fn delegating_propagates_client_recoverability() {
    let e = TestOpError::Client(TestClientError);
    assert_eq!(e.recoverability(), Recoverability::UserAction);
}

// --- as_error downcasting ---

#[test]
fn as_error_finds_inner_type() {
    let op: &dyn ErrorCompose =
        &TestOpError::Data(TestDataError { message: "oops".into(), hint: Some("fix it".into()) });
    let inner = op.as_error::<TestDataError>();
    assert!(inner.is_some());
    assert_eq!(inner.unwrap().hint.as_deref(), Some("fix it"));
}

#[test]
fn as_error_returns_none_for_wrong_type() {
    let op: &dyn ErrorCompose = &TestOpError::Client(TestClientError);
    assert!(op.as_error::<TestDataError>().is_none());
}

// --- BatchError worst-case aggregation ---

#[test]
fn batch_fix_domain_worst_case() {
    let batch = BatchError::new(vec![
        TestOpError::Client(TestClientError),
        TestOpError::Data(TestDataError { message: "x".into(), hint: None }),
    ]);
    // Data > Client
    assert_eq!(batch.fix_domain(), FixDomain::Data);
}

#[test]
fn batch_recoverability_worst_case() {
    let batch = BatchError::new(vec![
        TestOpError::Client(TestClientError),       // UserAction
        TestOpError::Data(TestDataError { message: "x".into(), hint: None }), // OperatorAction
    ]);
    // OperatorAction > UserAction
    assert_eq!(batch.recoverability(), Recoverability::OperatorAction);
}

#[test]
fn batch_empty_defaults_to_pari_not_recoverable() {
    let batch: BatchError<TestOpError> = BatchError::new(vec![]);
    assert_eq!(batch.fix_domain(), FixDomain::Pari);
    assert_eq!(batch.recoverability(), Recoverability::NotRecoverable);
}

// --- OTelEmit compile test (emit() exists and is callable) ---

#[derive(Error, Debug, ErrorCompose, OTelEmit)]
#[error("test emit")]
#[compose(fix = Client, recoverability = UserAction)]
#[otel(error_type = "test_emit")]
struct TestEmitError {
    #[otel(field = "test.field")]
    pub detail: String,
}

#[test]
fn otel_emit_compiles_and_is_callable() {
    let e = TestEmitError { detail: "hello".into() };
    e.emit();  // must compile; event is a no-op if no tracing subscriber
}
