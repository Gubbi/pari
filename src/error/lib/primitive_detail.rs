#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrimitiveDetail {
    pub field_name: &'static str,
    pub value: String,
}
