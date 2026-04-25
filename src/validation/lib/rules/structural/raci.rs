use crate::{entity::types::Raci, error::primitive::PrimitiveError};

/// `Raci.responsible` must be non-empty.
pub fn raci_structural(value: &Raci) -> Vec<PrimitiveError> {
    if value.responsible.is_empty() {
        vec![PrimitiveError::empty_required_value(
            "responsible must not be empty",
            Some(".responsible"),
            "raci_structural",
        )]
    } else {
        vec![]
    }
}
