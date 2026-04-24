//! `ErrorLocation` — the source location a primitive error points at.
//!
//! By default, every primitive error captures its construction site. That is
//! almost always the right answer and requires no work from the caller.
//!
//! Some primitives represent failures whose *interesting* location is not
//! where the error struct is built — a codec detecting a malformed line in a
//! document wants the location to point at the document, not the decoder.
//! Those call sites construct an `ErrorLocation` explicitly and pass it to the
//! primitive's `new_with_location(...)` constructor.
//!
//! Emitted as OTel `code.filepath` / `code.lineno` / `code.column`.

/// The source location an error points at. Used as a structural diagnostic on
/// every primitive error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ErrorLocation {
    pub file: String,
    pub line: u32,
    pub column: u32,
}

impl ErrorLocation {
    /// Capture the caller's source location.
    ///
    /// Relies on `#[track_caller]`, so the returned location is the site of the
    /// *call to `caller()`*, not the body of this function. Primitive-error
    /// constructors use this to auto-capture their construction site.
    #[track_caller]
    pub fn caller() -> Self {
        let location = std::panic::Location::caller();
        Self {
            file: location.file().to_string(),
            line: location.line(),
            column: location.column(),
        }
    }
}
