//! Cross-entity validation stubs.
//!
//! All functions return `vec![]` (no violations) until a substrate load path is available.

use super::RuleViolation;

/// Check that a referenced entity exists in the store.
/// Stub: always returns no violations.
pub fn ref_exists() -> Vec<RuleViolation> {
    vec![]
}

/// Check that all refs in a collection exist in the store.
/// Stub: always returns no violations.
pub fn all_refs_exist() -> Vec<RuleViolation> {
    vec![]
}

/// Check that all hook call inputs match the hook's declared inputs.
/// Stub: always returns no violations.
pub fn hook_call_inputs_valid() -> Vec<RuleViolation> {
    vec![]
}

/// Check that all role refs in a Raci exist in the store.
/// Stub: always returns no violations.
pub fn raci_roles_exist() -> Vec<RuleViolation> {
    vec![]
}
