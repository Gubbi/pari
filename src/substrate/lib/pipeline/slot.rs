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

#[cfg(test)]
mod tests {
    //! `FlattenRule::match_len` is the building block of longest-
    //! prefix-match resolution between flatten slots in the same
    //! asset. `Some(len)` means "this rule absorbs this key, and the
    //! rule's prefix is `len` characters" — callers use the length
    //! to pick the most-specific matching rule among several.

    use super::*;

    #[test]
    fn prefix_matches_returns_prefix_length() {
        let rule = FlattenRule::Prefix("x-");
        assert_eq!(rule.match_len("x-color"), Some(2));
    }

    #[test]
    fn prefix_no_match_returns_none() {
        let rule = FlattenRule::Prefix("x-");
        assert_eq!(rule.match_len("color"), None);
    }

    #[test]
    fn prefix_exact_key_matches() {
        // The literal prefix string itself is a match — the key
        // happens to be exactly the prefix with nothing after.
        let rule = FlattenRule::Prefix("x-");
        assert_eq!(rule.match_len("x-"), Some(2));
    }

    #[test]
    fn prefix_case_sensitive() {
        let rule = FlattenRule::Prefix("x-");
        assert_eq!(rule.match_len("X-color"), None);
    }

    #[test]
    fn empty_prefix_matches_everything_with_zero_length() {
        // An empty prefix always matches; length is 0 so it loses
        // every longest-match contest against any non-empty rule.
        // Schemas should not declare empty prefixes, but the helper
        // shouldn't panic if they do.
        let rule = FlattenRule::Prefix("");
        assert_eq!(rule.match_len("anything"), Some(0));
        assert_eq!(rule.match_len(""), Some(0));
    }

    #[test]
    fn longer_prefix_yields_longer_length() {
        // The contract that callers depend on: a more-specific
        // prefix produces a strictly larger match length than a
        // less-specific one for the same key.
        let general = FlattenRule::Prefix("x-");
        let specific = FlattenRule::Prefix("x-doc-");
        let key = "x-doc-rationale";
        assert!(general.match_len(key).unwrap() < specific.match_len(key).unwrap());
    }

    #[test]
    fn empty_key_only_matches_empty_prefix() {
        assert_eq!(FlattenRule::Prefix("").match_len(""), Some(0));
        assert_eq!(FlattenRule::Prefix("x-").match_len(""), None);
    }
}
