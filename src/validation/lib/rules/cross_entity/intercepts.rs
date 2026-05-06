use std::collections::HashMap;

use crate::{
    entity::{entities::hook::Hook, types::HookCall, AnyEntityRef, Entity},
    error::primitive::PrimitiveError,
    workspace::Workspace,
};

/// Checks that the hook ref in every `HookCall` in `intercepts` exists in the store.
///
/// `field` is the field name used in sub-paths, e.g. `"intercepts"`.
pub async fn intercept_hooks_exist<T>(
    workspace: &Workspace,
    intercepts: HashMap<T, HookCall>,
    field: &str,
) -> Vec<PrimitiveError>
where
    T: std::hash::Hash + Eq + std::fmt::Debug,
{
    let mut errors = vec![];
    for hook_call in intercepts.values() {
        let any_ref: AnyEntityRef = AnyEntityRef::Hook(hook_call.hook.clone());
        let id = any_ref.id().to_owned();
        match workspace.has_any(any_ref).await {
            Ok(false) => errors.push(PrimitiveError::referenced_entity_absent(
                format!("hook '{id}' referenced in '{field}' does not exist"),
                format!("{field}.hook"),
                id,
            )),
            Ok(true) => {}
            Err(_) => {}
        }
    }
    errors
}

/// Validates input bindings for every `HookCall` in `intercepts`:
/// - No unknown keys in `with` (must match a declared `HookInput.name`)
/// - All required inputs must have a binding in `with`
///
/// `field` is the field name used in sub-paths, e.g. `"intercepts"`.
pub async fn intercept_inputs_valid<T>(
    workspace: &Workspace,
    intercepts: HashMap<T, HookCall>,
    field: &str,
) -> Vec<PrimitiveError>
where
    T: std::hash::Hash + Eq + std::fmt::Debug,
{
    let mut errors = vec![];
    for hook_call in intercepts.values() {
        let any_ref: AnyEntityRef = AnyEntityRef::Hook(hook_call.hook.clone());
        let tracked = match workspace.resolve_any(any_ref).await {
            Ok(t) => t,
            Err(_) => continue, // hook missing — caught by intercept_hooks_exist
        };
        let tracked_hook = match Hook::extract(&tracked) {
            Some(h) => h,
            None => continue,
        };
        let declared_inputs = match tracked_hook.inputs.get() {
            Some(opt) => match opt {
                Some(inputs) => inputs.clone(),
                None => vec![],
            },
            None => continue, // field not loaded — skip
        };

        let bound: HashMap<String, String> = match &hook_call.with {
            Some(w) => w.clone(),
            None => HashMap::new(),
        };

        // No unknown keys
        for key in bound.keys() {
            if !declared_inputs.iter().any(|i| &i.name == key) {
                errors.push(PrimitiveError::referenced_entity_absent(
                    format!("unknown input key '{key}' not declared by hook"),
                    format!("{field}.with.{key}"),
                    key.clone(),
                ));
            }
        }

        // All required inputs must be bound
        for input in &declared_inputs {
            if input.required && !bound.contains_key(&input.name) {
                errors.push(PrimitiveError::empty_required_value(
                    format!("required input '{}' has no binding", input.name),
                    Some(format!("{field}.with.{}", input.name)),
                    "required_input_missing",
                ));
            }
        }
    }
    errors
}
