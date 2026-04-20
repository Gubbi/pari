//! `SubstrateError` — substrate boundary error enum.

use crate::{
    error::{primitive::PrimitiveError, ErrorCompose, FixDomain, OTelEmit, Recoverability},
};

#[derive(Debug)]
pub enum SubstrateError {
    InvalidPersistenceLayout {
        source: PrimitiveError,
        span_trace: tracing_error::SpanTrace,
        backtrace: std::backtrace::Backtrace,
    },
    UnpersistableDefinition {
        source: PrimitiveError,
        span_trace: tracing_error::SpanTrace,
        backtrace: std::backtrace::Backtrace,
    },
    CorruptPersistenceState {
        source: PrimitiveError,
        span_trace: tracing_error::SpanTrace,
        backtrace: std::backtrace::Backtrace,
    },
}

impl SubstrateError {
    pub fn invalid_persistence_layout(source: PrimitiveError) -> Self {
        Self::InvalidPersistenceLayout {
            source,
            span_trace: tracing_error::SpanTrace::capture(),
            backtrace: std::backtrace::Backtrace::capture(),
        }
    }

    pub fn unpersistable_definition(source: PrimitiveError) -> Self {
        Self::UnpersistableDefinition {
            source,
            span_trace: tracing_error::SpanTrace::capture(),
            backtrace: std::backtrace::Backtrace::capture(),
        }
    }

    pub fn corrupt_persistence_state(source: PrimitiveError) -> Self {
        Self::CorruptPersistenceState {
            source,
            span_trace: tracing_error::SpanTrace::capture(),
            backtrace: std::backtrace::Backtrace::capture(),
        }
    }
}

impl std::fmt::Display for SubstrateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidPersistenceLayout { source, .. } => {
                write!(f, "invalid persistence layout: {source}")
            }
            Self::UnpersistableDefinition { source, .. } => {
                write!(f, "unpersistable definition: {source}")
            }
            Self::CorruptPersistenceState { source, .. } => {
                write!(f, "corrupt persistence state: {source}")
            }
        }
    }
}

impl std::error::Error for SubstrateError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::InvalidPersistenceLayout { source, .. }
            | Self::UnpersistableDefinition { source, .. }
            | Self::CorruptPersistenceState { source, .. } => Some(source),
        }
    }
}

impl ErrorCompose for SubstrateError {
    fn fix_domain(&self) -> FixDomain {
        match self {
            Self::InvalidPersistenceLayout { .. } | Self::UnpersistableDefinition { .. } => {
                FixDomain::Data
            }
            Self::CorruptPersistenceState { .. } => FixDomain::Infra,
        }
    }

    fn recoverability(&self) -> Recoverability {
        Recoverability::OperatorAction
    }

    fn as_any_inner(&self) -> Option<&dyn std::any::Any> {
        match self {
            Self::InvalidPersistenceLayout { source, .. }
            | Self::UnpersistableDefinition { source, .. }
            | Self::CorruptPersistenceState { source, .. } => Some(source as &dyn std::any::Any),
        }
    }
}

impl OTelEmit for SubstrateError {
    fn emit(&self) {
        match self {
            Self::InvalidPersistenceLayout {
                source,
                span_trace,
                backtrace,
            } => {
                tracing::error!(
                    exception.type = "invalid_persistence_layout",
                    exception.message = %"invalid persistence layout",
                    exception.stacktrace = %backtrace,
                    span_trace = %span_trace,
                );
                source.emit();
            }
            Self::UnpersistableDefinition {
                source,
                span_trace,
                backtrace,
            } => {
                tracing::error!(
                    exception.type = "unpersistable_definition",
                    exception.message = %"unpersistable definition",
                    exception.stacktrace = %backtrace,
                    span_trace = %span_trace,
                );
                source.emit();
            }
            Self::CorruptPersistenceState {
                source,
                span_trace,
                backtrace,
            } => {
                tracing::error!(
                    exception.type = "corrupt_persistence_state",
                    exception.message = %"corrupt persistence state",
                    exception.stacktrace = %backtrace,
                    span_trace = %span_trace,
                );
                source.emit();
            }
        }
    }
}
