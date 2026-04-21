use crate::error::primitive::PrimitiveError;

pub fn each_item_non_empty_str(value: &Option<Vec<String>>) -> Vec<PrimitiveError> {
    match value {
        None => vec![],
        Some(items) => items
            .iter()
            .enumerate()
            .flat_map(|(i, s)| {
                if s.trim().is_empty() {
                    vec![PrimitiveError::empty_required_value(
                        "must not be empty",
                        Some(format!("[{i}]")),
                        "non_empty",
                    )]
                } else {
                    vec![]
                }
            })
            .collect(),
    }
}
