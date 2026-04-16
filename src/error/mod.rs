//! Cross-cutting error handling infrastructure.
//!
//! Classification enums, `ErrorCompose` trait, `OTelEmit` trait, and `BatchError<E>`.

pub mod pari_error;

// ---------------------------------------------------------------------------
// Classification types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FixDomain {
    /// Fix is in the caller's input or usage.
    Client,
    /// Fix requires repairing stored content (corrupt or malformed).
    Data,
    /// Fix is in the underlying infrastructure (permissions, disk, network).
    Infra,
    /// Fix is in Pari's code (invariant violated, logic bug).
    Pari,
}

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

// ---------------------------------------------------------------------------
// ErrorCompose trait
// ---------------------------------------------------------------------------

pub trait ErrorCompose: sealed::AsAny + std::error::Error + Send + Sync + 'static {
    fn fix_domain(&self) -> FixDomain;
    fn recoverability(&self) -> Recoverability;
    fn severity(&self) -> Severity {
        Severity::from_classification(self.fix_domain(), self.recoverability())
    }
    /// For delegating enums: returns `&dyn Any` of the wrapped inner error.
    /// Default returns `None`; the `#[derive(ErrorCompose)]` macro overrides this for enums.
    fn as_any_inner(&self) -> Option<&dyn std::any::Any> {
        None
    }
}

impl dyn ErrorCompose {
    /// Downcast to a concrete error type.
    /// Checks the current node first, then the inner wrapped value for delegating enums.
    pub fn as_error<E: 'static>(&self) -> Option<&E> {
        if let Some(e) = self.as_any().downcast_ref::<E>() {
            return Some(e);
        }
        self.as_any_inner()?.downcast_ref::<E>()
    }
}

mod sealed {
    pub trait AsAny: 'static {
        fn as_any(&self) -> &dyn std::any::Any;
    }
    impl<T: 'static> AsAny for T {
        fn as_any(&self) -> &dyn std::any::Any {
            self
        }
    }
}

// ---------------------------------------------------------------------------
// OTelEmit trait
// ---------------------------------------------------------------------------

pub trait OTelEmit {
    /// Emit a structured OTel event. Cascades to inner errors via `source()`.
    fn emit(&self);
}

// ---------------------------------------------------------------------------
// BatchError<E>
// ---------------------------------------------------------------------------

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
