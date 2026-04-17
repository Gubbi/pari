//! Structural validation schema for [`Hook`].

use super::{
    kebab_case_id, non_empty_list, non_empty_str, unique_by, x_prefix_keys, AnyStructuralRule,
    RuleViolation, ValidationSchema,
};
use crate::entity::entities::hook::{Hook, HookInput, TrackedHook};

fn opt_non_empty_str(value: &Option<String>) -> Vec<RuleViolation> {
    match value {
        None => vec![],
        Some(s) => non_empty_str(s),
    }
}

fn hook_inputs_structural(value: &Option<Vec<HookInput>>) -> Vec<RuleViolation> {
    match value {
        None => vec![],
        Some(inputs) => {
            let mut v = vec![];
            // Each input name must be non-empty
            for (i, input) in inputs.iter().enumerate() {
                v.extend(
                    non_empty_str(&input.name)
                        .into_iter()
                        .map(|viol| RuleViolation::sub(format!("[{i}].name"), viol.message)),
                );
            }
            // Input names must be unique
            v.extend(
                unique_by(inputs, |inp| inp.name.clone())
                    .into_iter()
                    .map(|viol| RuleViolation {
                        sub_path: viol.sub_path.map(|p| p + ".name"),
                        ..viol
                    }),
            );
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
                        violations.extend(
                            non_empty_str(instr)
                                .into_iter()
                                .map(|viol| RuleViolation::sub(format!("[{i}]"), viol.message)),
                        );
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
