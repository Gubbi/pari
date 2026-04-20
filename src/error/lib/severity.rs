use super::{FixDomain, Recoverability};

/// Severity is derived, never declared.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    Warn,
    Error,
}

impl Severity {
    pub fn from_classification(fix: FixDomain, recoverability: Recoverability) -> Self {
        match (fix, recoverability) {
            (FixDomain::Pari, Recoverability::NotRecoverable) => Severity::Error,
            (FixDomain::Data, Recoverability::OperatorAction) => Severity::Error,
            (FixDomain::Infra, Recoverability::OperatorAction) => Severity::Error,
            (FixDomain::Infra, Recoverability::Retryable) => Severity::Warn,
            (FixDomain::Client, Recoverability::UserAction) => Severity::Warn,
            _ => Severity::Error,
        }
    }
}
