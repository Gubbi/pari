/// Marker for backend-specific slot kinds used in `FieldMapping`
/// entries. Slots let a backend attach positional or structural
/// information to each field — the default `ValueSlot` covers backends
/// whose assets carry one value per field.
pub trait Slot: Copy + 'static {}

/// Default single-slot marker used by backends that do not need
/// positional slot specialisation.
///
/// `Value` is the literal-key slot — the entry's `field.key` looks up
/// directly in the wire JSON. `Flattened` is the open-ended bag slot —
/// it absorbs unclaimed top-level wire keys whose name matches the
/// rule. Multiple `Flattened` slots in the same asset are allowed;
/// longest-prefix-match wins.
#[derive(Clone, Copy)]
pub enum ValueSlot {
    Value,
    Flattened(FlattenRule),
}

impl Slot for ValueSlot {}

/// Selects which unclaimed top-level wire keys a flattened slot
/// absorbs. Disjointness across slots in the same asset is *not*
/// required — overlapping prefixes resolve by longest-match.
///
/// Today the only variant is `Prefix`; richer matchers (regex,
/// suffix, …) can be added without breaking existing entries.
#[derive(Clone, Copy)]
pub enum FlattenRule {
    /// Match wire keys whose name starts with this exact (case-sensitive)
    /// prefix.
    Prefix(&'static str),
}

impl FlattenRule {
    /// Length of the matching prefix when `key` is absorbed by this
    /// rule, or `None` if the rule rejects `key`.
    pub fn match_len(&self, key: &str) -> Option<usize> {
        match self {
            FlattenRule::Prefix(p) => key.starts_with(p).then_some(p.len()),
        }
    }
}
