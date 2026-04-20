#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Recoverability {
    /// Transient failure — retry automatically after backoff.
    Retryable,
    /// Caller must fix their input or definition, then retry.
    UserAction,
    /// Operator must fix infrastructure or data, then retry.
    OperatorAction,
    /// Code invariant violated — do not retry, escalate to developer.
    NotRecoverable,
}
