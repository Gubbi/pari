//! `CodecError` — encode/decode failure scoped to a named field.

use pari_macros::{ErrorCompose, OTelEmit};

#[derive(thiserror::Error, Debug, ErrorCompose, OTelEmit)]
#[error("codec error on field '{field}': {message}")]
#[compose(fix = Data, recoverability = OperatorAction)]
#[otel(error_type = "codec_error")]
pub struct CodecError {
    #[otel(field = "error.field")]
    pub field:   String,
    #[otel(field = "error.message")]
    pub message: String,
    pub span_trace: tracing_error::SpanTrace,
    pub backtrace:  std::backtrace::Backtrace,
}

impl CodecError {
    pub fn new(field: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            field:      field.into(),
            message:    message.into(),
            span_trace: tracing_error::SpanTrace::capture(),
            backtrace:  std::backtrace::Backtrace::capture(),
        }
    }
}
