pub trait OTelEmit {
    /// Emit a structured OTel event. Cascades to inner errors via `source()`.
    fn emit(&self);
}
