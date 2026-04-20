use super::{ErrorCompose, FixDomain, OTelEmit, Recoverability};

/// Wraps a collection of failures from a single operation.
/// Classification properties are aggregated worst-case across all inner errors.
#[derive(Debug)]
pub struct BatchError<E: ErrorCompose + std::fmt::Debug> {
    pub errors: Vec<E>,
}

impl<E: ErrorCompose + std::fmt::Debug> BatchError<E> {
    pub fn new(errors: Vec<E>) -> Self {
        Self { errors }
    }
}

impl<E: ErrorCompose + std::fmt::Debug + std::fmt::Display> std::fmt::Display for BatchError<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} error(s)", self.errors.len())?;
        for (i, e) in self.errors.iter().enumerate() {
            write!(f, "; [{i}] {e}")?;
        }
        Ok(())
    }
}

impl<E: ErrorCompose + std::fmt::Debug + std::fmt::Display> std::error::Error for BatchError<E> {}

impl<E: ErrorCompose + std::fmt::Debug + std::fmt::Display> ErrorCompose for BatchError<E> {
    fn fix_domain(&self) -> FixDomain {
        self.errors
            .iter()
            .map(|e| e.fix_domain())
            .max_by_key(|d| match d {
                FixDomain::Pari => 3,
                FixDomain::Data => 2,
                FixDomain::Infra => 1,
                FixDomain::Client => 0,
            })
            .unwrap_or(FixDomain::Pari)
    }

    fn recoverability(&self) -> Recoverability {
        self.errors
            .iter()
            .map(|e| e.recoverability())
            .max_by_key(|r| match r {
                Recoverability::NotRecoverable => 3,
                Recoverability::OperatorAction => 2,
                Recoverability::UserAction => 1,
                Recoverability::Retryable => 0,
            })
            .unwrap_or(Recoverability::NotRecoverable)
    }
}

impl<E: ErrorCompose + OTelEmit + std::fmt::Debug + std::fmt::Display> OTelEmit for BatchError<E> {
    fn emit(&self) {
        for e in &self.errors {
            e.emit();
        }
    }
}
