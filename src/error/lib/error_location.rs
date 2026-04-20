#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ErrorLocation {
    pub file: String,
    pub line: u32,
    pub column: u32,
}

impl ErrorLocation {
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
