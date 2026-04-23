//! `CollectRefs` — uniform ref-extraction across entity-layer types.
//!
//! Any type that may contain cross-entity refs implements this trait so that
//! `all_refs()` and `all_refs_with_paths()` on tracked entities can traverse
//! nested structures without entity-specific scanning code in other layers.
//!
//! # Convention
//!
//! `collect_refs(prefix, refs)` appends `(sub_path, AnyEntityRef)` pairs into
//! `refs`. `prefix` is the dot-notation path from the current field to the
//! enclosing field name, e.g. `"raci"`. The impl appends child segments as
//! needed: `"raci.accountable"`, `"raci.responsible[0]"`, etc.
//!
//! Types with no entity refs (plain strings, state entries, triggers, …)
//! implement the blanket no-op and contribute nothing.

use std::collections::HashMap;

use indexmap::IndexMap;

use super::{AnyEntityRef, Entity, EntityRef, ParentKind};

pub trait CollectRefs {
    fn collect_refs(&self, prefix: &str, refs: &mut Vec<(String, AnyEntityRef)>);
}

// ---------------------------------------------------------------------------
// EntityRef<T, P> — the leaf: push itself
// ---------------------------------------------------------------------------

impl<T: Entity<Parent = P>, P: ParentKind> CollectRefs for EntityRef<T, P> {
    fn collect_refs(&self, prefix: &str, refs: &mut Vec<(String, AnyEntityRef)>) {
        refs.push((prefix.to_owned(), T::to_any_ref(self)));
    }
}

// ---------------------------------------------------------------------------
// Standard container wrappers
// ---------------------------------------------------------------------------

impl<T: CollectRefs> CollectRefs for Option<T> {
    fn collect_refs(&self, prefix: &str, refs: &mut Vec<(String, AnyEntityRef)>) {
        if let Some(inner) = self {
            inner.collect_refs(prefix, refs);
        }
    }
}

impl<T: CollectRefs> CollectRefs for Vec<T> {
    fn collect_refs(&self, prefix: &str, refs: &mut Vec<(String, AnyEntityRef)>) {
        for (i, item) in self.iter().enumerate() {
            item.collect_refs(&format!("{prefix}[{i}]"), refs);
        }
    }
}

/// HashMap: keys and values may each carry refs.
impl<K: CollectRefs, V: CollectRefs> CollectRefs for HashMap<K, V> {
    fn collect_refs(&self, prefix: &str, refs: &mut Vec<(String, AnyEntityRef)>) {
        for (k, v) in self {
            k.collect_refs(prefix, refs);
            v.collect_refs(prefix, refs);
        }
    }
}

/// IndexMap with String keys: folds the key into the child prefix so that
/// e.g. `steps.WriteProposal.entity_ref` is produced from `steps["WriteProposal"].entity_ref`.
impl<V: CollectRefs> CollectRefs for IndexMap<String, V> {
    fn collect_refs(&self, prefix: &str, refs: &mut Vec<(String, AnyEntityRef)>) {
        for (key, val) in self {
            let child = format!("{prefix}.{key}");
            val.collect_refs(&child, refs);
        }
    }
}

// ---------------------------------------------------------------------------
// No-op blanket for primitive / plain-data types
// ---------------------------------------------------------------------------

macro_rules! no_op_collect_refs {
    ($($t:ty),* $(,)?) => {
        $(
            impl CollectRefs for $t {
                fn collect_refs(&self, _prefix: &str, _refs: &mut Vec<(String, AnyEntityRef)>) {}
            }
        )*
    };
}

no_op_collect_refs!(
    String,
    bool,
    u8,
    u16,
    u32,
    u64,
    u128,
    i8,
    i16,
    i32,
    i64,
    i128,
    f32,
    f64,
    serde_json::Value,
);
