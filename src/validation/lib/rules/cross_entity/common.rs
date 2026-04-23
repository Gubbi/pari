//! Cross-entity validation helpers.
//!
//! `check_refs` is the single shared primitive: given a list of `(field_path,
//! AnyEntityRef)` pairs produced by `CollectRefs`, it queries the store for each
//! ref and returns a `ReferencedEntityAbsent` error for every one that is missing.
//!
//! Schema files build rules by collecting refs from a field value with
//! `CollectRefs::collect_refs` before the async boundary, then calling
//! `check_refs` inside the async block.

use crate::{
    entity::{collect_refs::CollectRefs, AnyEntityRef},
    error::primitive::PrimitiveError,
    workspace::EntityClient,
};

/// Builds a cross-entity rule that collects all entity refs from a tracked
/// entity field via `CollectRefs` and checks their existence in the store.
///
/// ```ignore
/// cross_entity.insert("raci", vec![ref_check_rule!(TrackedWorkflow, raci)]);
/// ```
#[macro_export]
macro_rules! ref_check_rule {
    ($TrackedType:ty, $field:ident) => {
        Box::new(|e: &$TrackedType| {
            let pairs = $crate::validation::lib::rules::cross_entity::common::collect_field_refs(
                e.$field.get(),
                stringify!($field),
            );
            Box::pin(async move {
                $crate::validation::lib::rules::cross_entity::common::check_refs(pairs).await
            })
        })
    };
}

/// Checks each `(field_path, ref)` pair against the store.
///
/// Returns `ReferencedEntityAbsent` for every ref that does not exist.
/// Store transport errors are silently skipped — the store layer surfaces them
/// independently.
pub async fn check_refs(pairs: Vec<(String, AnyEntityRef)>) -> Vec<PrimitiveError> {
    let mut errors = Vec::new();
    for (path, any_ref) in pairs {
        let id = any_ref.id().to_owned();
        match EntityClient::has_ref(any_ref).await {
            Ok(false) => errors.push(PrimitiveError::referenced_entity_absent(
                format!("referenced entity '{id}' at '{path}' does not exist"),
                path,
                id,
            )),
            Ok(true) => {}
            Err(_) => {} // store unavailable — skip, surfaced by store layer
        }
    }
    errors
}

/// Collects refs from a field value and checks them all.
///
/// Convenience for the common single-field rule body:
/// ```ignore
/// let pairs = collect_field_refs(e.raci.get(), "raci");
/// Box::pin(async move { check_refs(pairs).await })
/// ```
pub fn collect_field_refs<T: CollectRefs>(
    value: Option<&T>,
    field_name: &str,
) -> Vec<(String, AnyEntityRef)> {
    let mut pairs = Vec::new();
    if let Some(v) = value {
        CollectRefs::collect_refs(v, field_name, &mut pairs);
    }
    pairs
}
