//! `ExecutorError` — a single asset request failed at the I/O layer.

use pari_macros::{ErrorCompose, OTelEmit};

#[derive(thiserror::Error, Debug, ErrorCompose, OTelEmit)]
#[error("executor error at '{location}': {message}")]
#[compose(fix = Infra, recoverability = OperatorAction)]
#[otel(error_type = "executor_error")]
pub struct ExecutorError {
    #[otel(field = "fs.path")]
    pub location: String,
    #[otel(field = "error.message")]
    pub message:  String,
    pub span_trace: tracing_error::SpanTrace,
    pub backtrace:  std::backtrace::Backtrace,
}

impl ExecutorError {
    pub fn new(location: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            location:   location.into(),
            message:    message.into(),
            span_trace: tracing_error::SpanTrace::capture(),
            backtrace:  std::backtrace::Backtrace::capture(),
        }
    }
}
