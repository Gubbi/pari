//! Cross-entity validation stubs — pending implementation.
//!
//! These rules run on committed/loaded entities within the store, using
//! tracked entity accessors and ref checks. Full implementations are deferred.

use crate::error::primitive::PrimitiveError;

pub fn ref_exists() -> Vec<PrimitiveError> {
    vec![]
}

pub fn all_refs_exist() -> Vec<PrimitiveError> {
    vec![]
}

pub fn hook_call_inputs_valid() -> Vec<PrimitiveError> {
    vec![]
}

pub fn raci_roles_exist() -> Vec<PrimitiveError> {
    vec![]
}
