//! Path and location-resolution primitive errors.

use pari_macros::primitive_with_fields;

/// A path template could not be resolved into a concrete asset location.
#[primitive_with_fields]
pub struct PathResolutionFailed {
    path_template: String,
    reason: String,
}

/// A configured root path was syntactically or semantically unusable.
#[primitive_with_fields]
pub struct InvalidRootPath {
    root: String,
}

/// A path template referenced a placeholder that was not available in the input projection.
#[primitive_with_fields]
pub struct UnresolvedTemplatePlaceholder {
    path_template: String,
    placeholder: String,
}

/// Parent-derived path data required by a template was missing.
#[primitive_with_fields]
pub struct MissingParentBaseData {
    path_template: String,
    parent_kind: String,
}

/// A configured path template was invalid before resolution could succeed.
#[primitive_with_fields]
pub struct InvalidPathTemplate {
    path_template: String,
}

/// Path resolution would have produced a location outside the permitted substrate root.
#[primitive_with_fields]
pub struct PathEscapesSubstrateRoot {
    root: String,
    resolved_path: String,
}
