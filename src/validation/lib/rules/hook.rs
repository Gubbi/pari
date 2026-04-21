use super::{
    super::schema::{AnyStructuralRule, ValidationSchema},
    structural::{
        hook::hook_inputs_structural,
        primitives::{
            kebab_case_id, non_empty_list, non_empty_str, opt_non_empty_str, x_prefix_keys,
        },
    },
};
use crate::{
    entity::entities::hook::{Hook, TrackedHook},
    error::primitive::PrimitiveError,
};

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
