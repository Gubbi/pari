//! Structural validation schema for [`Hook`].

use super::{
    kebab_case_id, non_empty_list, non_empty_str, x_prefix_keys, AnyStructuralRule,
    ValidationSchema,
};
use crate::entity::entities::hook::{Hook, HookInput, TrackedHook};
use crate::error::primitive::PrimitiveError;

fn opt_non_empty_str(value: &Option<String>) -> Vec<PrimitiveError> {
    match value {
        None => vec![],
        Some(s) => non_empty_str(s),
    }
}

fn hook_inputs_structural(value: &Option<Vec<HookInput>>) -> Vec<PrimitiveError> {
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

pub fn hook_validation_schema() -> ValidationSchema<Hook> {
    let mut structural: std::collections::HashMap<&'static str, Vec<AnyStructuralRule<Hook>>> =
        std::collections::HashMap::new();

    structural.insert(
        "entity_ref",
        vec![Box::new(|e: &TrackedHook| kebab_case_id(&e.entity_ref))],
    );

    structural.insert(
        "name",
        vec![Box::new(|e: &TrackedHook| {
            e.name.get().map(|v| non_empty_str(v)).unwrap_or_default()
        })],
    );

    structural.insert(
        "description",
        vec![Box::new(|e: &TrackedHook| {
            e.description
                .get()
                .map(|v| opt_non_empty_str(v))
                .unwrap_or_default()
        })],
    );

    structural.insert(
        "instructions",
        vec![Box::new(|e: &TrackedHook| {
            e.instructions
                .get()
                .map(|v| {
                    let mut violations = non_empty_list(v.as_slice());
                    for (i, instr) in v.iter().enumerate() {
                        if instr.trim().is_empty() {
                            violations.push(PrimitiveError::empty_required_value(
                                "must not be empty",
                                Some(format!("[{i}]")),
                                "non_empty",
                            ));
                        }
                    }
                    violations
                })
                .unwrap_or_default()
        })],
    );

    structural.insert(
        "inputs",
        vec![Box::new(|e: &TrackedHook| {
            e.inputs
                .get()
                .map(|v| hook_inputs_structural(v))
                .unwrap_or_default()
        })],
    );

    structural.insert(
        "extensions",
        vec![Box::new(|e: &TrackedHook| {
            e.extensions
                .get()
                .map(|v| x_prefix_keys(v))
                .unwrap_or_default()
        })],
    );

    ValidationSchema {
        structural,
        semantic: std::collections::HashMap::new(),
        cross_entity: std::collections::HashMap::new(),
    }
}
