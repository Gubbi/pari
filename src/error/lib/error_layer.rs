#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorLayer {
    Primitive,
    Activity,
    Intermediary,
    Job,
}
