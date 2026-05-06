use crate::{entity::entities::hook::HookInput, error::primitive::PrimitiveError};

pub fn hook_inputs_structural(value: &Option<Vec<HookInput>>) -> Vec<PrimitiveError> {
    match value {
        None => vec![],
        Some(inputs) => {
            let mut v = vec![];
            for (i, input) in inputs.iter().enumerate() {
                if input.name.trim().is_empty() {
                    v.push(PrimitiveError::empty_required_value(
                        "must not be empty",
                        Some(format!("[{i}].name")),
                        "non_empty",
                    ));
                }
                if let Some(desc) = &input.description {
                    if desc.trim().is_empty() {
                        v.push(PrimitiveError::empty_required_value(
                            "must not be empty",
                            Some(format!("[{i}].description")),
                            "non_empty",
                        ));
                    }
                }
            }
            let mut seen = std::collections::HashSet::new();
            for (i, inp) in inputs.iter().enumerate() {
                if !seen.insert(inp.name.clone()) {
                    v.push(PrimitiveError::duplicate_entry_violation(
                        "duplicate entry",
                        format!("[{i}].name"),
                        "unique",
                    ));
                }
            }
            v
        }
    }
}
