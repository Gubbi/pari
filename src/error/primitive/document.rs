//! Document and encoded-asset primitive errors.

use pari_macros::primitive_with_fields;

/// The selected schema slots cannot be encoded together into one valid asset body.
#[primitive_with_fields]
pub struct UnsupportedSlotComposition {
    slot: String,
    field: String,
}

/// Frontmatter content could not be serialized into the encoded document format.
#[primitive_with_fields]
pub struct FrontmatterSerializationFailed {
    field: String,
    reason: String,
}

/// A section payload could not be rendered into the target document body.
#[primitive_with_fields]
pub struct SectionRenderingFailed {
    section: String,
    field: String,
}

/// An encoded frontmatter block could not be parsed as a valid frontmatter structure.
#[primitive_with_fields]
pub struct MalformedFrontmatter {
    raw_snippet: String,
}

/// A frontmatter block was present but was not valid YAML.
#[primitive_with_fields]
pub struct InvalidYamlFrontmatter {
    raw_snippet: String,
}

/// Parsed YAML frontmatter could not be converted into the expected JSON shape.
#[primitive_with_fields]
pub struct InvalidYamlJsonConversion {
    raw_snippet: String,
}

/// A decoded section body did not match the shape required by the schema slot.
#[primitive_with_fields]
pub struct UnsupportedSectionBodyShape {
    section: String,
    body_shape: String,
}

/// Multiple headings or sections collapsed into the same reconstructed field mapping.
#[primitive_with_fields]
pub struct DuplicateHeadingCollision {
    heading: String,
}

/// The document body did not contain enough valid structure to reconstruct a schema slot.
#[primitive_with_fields]
pub struct UnreconstructableSchemaSlot {
    slot: String,
    field: String,
}
