//! Change tracking primitives for incremental persistence.
//!
//! [`Tracked<T>`] — newtype that marks itself dirty on mutable access.
//! [`TrackedMap<K,V>`] — `IndexMap`-backed map that tracks inserted, modified, and removed keys.
//! [`HasId`] — trait used by [`TrackedMap::from_vec`] to extract keys from plain types.

use std::{
    hash::Hash,
    ops::{Deref, DerefMut},
    sync::OnceLock,
    sync::atomic::{AtomicBool, Ordering},
};

use indexmap::IndexMap;

// ---------------------------------------------------------------------------
// HasId
// ---------------------------------------------------------------------------

/// Types that have a string identifier.  Used by [`TrackedMap::from_vec`] for
/// key extraction.
pub trait HasId {
    fn id(&self) -> &str;
}

// ---------------------------------------------------------------------------
// Tracked<T>
// ---------------------------------------------------------------------------

/// Newtype wrapper that tracks whether its inner value has been mutated.
///
/// A newly constructed `Tracked<T>` starts **dirty**.  Mutable access via
/// [`DerefMut`] marks the value dirty; [`reset_dirty`](Tracked::reset_dirty)
/// clears the flag without changing the value.
pub struct Tracked<T> {
    value: T,
    dirty: bool,
}

impl<T> Tracked<T> {
    /// Create a new `Tracked<T>` wrapping `value`.  Starts dirty.
    pub fn new(value: T) -> Self {
        Self { value, dirty: true }
    }

    /// Returns `true` if this value has been created or mutated since the last
    /// [`reset_dirty`](Tracked::reset_dirty).
    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    /// Clear the dirty flag.  Does not change the inner value.
    pub fn reset_dirty(&mut self) {
        self.dirty = false;
    }
}

impl<T> Deref for Tracked<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

impl<T> DerefMut for Tracked<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.dirty = true;
        &mut self.value
    }
}

impl<T: std::fmt::Display> std::fmt::Display for Tracked<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.value.fmt(f)
    }
}

// ---------------------------------------------------------------------------
// TrackedMap<K, V>
// ---------------------------------------------------------------------------

/// `IndexMap`-backed map with insertion-order preservation and three change-
/// tracking sets: `inserted`, `modified`, and `removed`.
///
/// ### Precedence
/// `inserted` takes precedence over `modified`.  A key inserted and then
/// mutated before [`reset_tracked`](TrackedMap::reset_tracked) is classified
/// as `Added`, not `Modified`.
///
/// ### Removed values
/// Deleted values are **retained** in `removed` until `reset_tracked()` is
/// called so that `collect_changes()` has full entity data for `Removed`
/// entries.
pub struct TrackedMap<K, V>
where
    K: Hash + Eq,
{
    data: IndexMap<K, V>,
    pub inserted: IndexMap<K, ()>,
    pub modified: IndexMap<K, ()>,
    pub removed: IndexMap<K, V>,
}

impl<K: Hash + Eq + Clone, V> Default for TrackedMap<K, V> {
    fn default() -> Self {
        Self::new()
    }
}

impl<K: Hash + Eq + Clone, V> TrackedMap<K, V> {
    /// Create an empty `TrackedMap`.
    pub fn new() -> Self {
        Self {
            data: IndexMap::new(),
            inserted: IndexMap::new(),
            modified: IndexMap::new(),
            removed: IndexMap::new(),
        }
    }

    /// Insert or replace a key-value pair.  Records the key in `inserted`;
    /// un-removes it if it was previously removed.
    pub fn insert(&mut self, key: K, value: V) {
        self.removed.shift_remove(&key);
        self.data.insert(key.clone(), value);
        self.inserted.insert(key, ());
    }

    /// Remove an entry.  The full value is retained in `removed` until
    /// [`reset_tracked`](TrackedMap::reset_tracked).
    pub fn remove(&mut self, key: &K) {
        if let Some(value) = self.data.shift_remove(key) {
            self.inserted.shift_remove(key);
            self.modified.shift_remove(key);
            self.removed.insert(key.clone(), value);
        }
    }

    /// Read-only access by key.
    pub fn get(&self, key: &K) -> Option<&V> {
        self.data.get(key)
    }

    /// Mutable access by key.  Records the key in `modified` unless it is
    /// already in `inserted` (inserted takes precedence).
    pub fn get_mut(&mut self, key: &K) -> Option<&mut V> {
        if self.data.contains_key(key) {
            if !self.inserted.contains_key(key) {
                self.modified.insert(key.clone(), ());
            }
            self.data.get_mut(key)
        } else {
            None
        }
    }

    /// Mutable iteration in insertion order.
    pub fn iter_mut(&mut self) -> indexmap::map::IterMut<'_, K, V> {
        self.data.iter_mut()
    }

    /// Iterate values in insertion order.
    pub fn values(&self) -> indexmap::map::Values<'_, K, V> {
        self.data.values()
    }

    /// Iterate keys in insertion order.
    pub fn keys(&self) -> indexmap::map::Keys<'_, K, V> {
        self.data.keys()
    }

    /// Returns `true` if any inserted, modified, or removed entries exist.
    pub fn has_changes(&self) -> bool {
        !self.inserted.is_empty() || !self.modified.is_empty() || !self.removed.is_empty()
    }

    /// Clear all tracking state and drop retained removed values.
    /// Only call after a successful `atomic_persist`.
    pub fn reset_tracked(&mut self) {
        self.inserted.clear();
        self.modified.clear();
        self.removed.clear();
    }

    /// Number of live entries (does not count removed entries).
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// Returns `true` if there are no live entries.
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }
}

impl<V> TrackedMap<String, V> {
    /// Convert a `Vec<S>` to a `TrackedMap<String, V>`.
    ///
    /// Keys are extracted via [`HasId::id`] on each source item **before**
    /// conversion; all entries start in the `inserted` set.
    pub fn from_vec<S: HasId>(items: Vec<S>, convert: impl Fn(S) -> V) -> Self {
        let mut map = Self::new();
        for item in items {
            let key = item.id().to_string();
            let value = convert(item);
            map.insert(key, value);
        }
        map
    }
}

// ---------------------------------------------------------------------------
// TrackedField<T>
// ---------------------------------------------------------------------------

/// OnceLock-backed field with an atomic dirty flag.
///
/// All domain fields on tracked entities are `Arc<TrackedField<T>>`.
/// The Arc enables cheap clone at checkout and COW replacement at mutation.
///
/// Two write paths:
/// - `initialize()` — write-once, used by the load path and deserializer.
///   Silently ignores if the field is already initialized.
/// - COW replacement — setters create a new `TrackedField::mutated(v)`
///   and swap the Arc pointer on the tracked entity. The old Arc is untouched.
pub struct TrackedField<T> {
    value: OnceLock<T>,
    dirty: AtomicBool,
}

impl<T> TrackedField<T> {
    /// Create an empty, clean field (uninitialized, not dirty).
    pub fn new() -> Self {
        Self { value: OnceLock::new(), dirty: AtomicBool::new(false) }
    }

    /// Create a field pre-seeded with `value` and marked dirty.
    /// Used by setters to build the replacement Arc before swapping.
    pub fn mutated(value: T) -> Self {
        let lock = OnceLock::new();
        let _ = lock.set(value);
        Self { value: lock, dirty: AtomicBool::new(true) }
    }

    /// Write-once initialization. Used by the load path and deserializer.
    /// If the field is already initialized, this is a no-op.
    pub fn initialize(&self, value: T) {
        let _ = self.value.set(value);
    }

    /// Returns the value if initialized, None otherwise.
    pub fn get(&self) -> Option<&T> {
        self.value.get()
    }

    /// Create a field pre-seeded with `value` and marked clean (dirty = false).
    /// Used by `From<PlainEntity>` — the entity is loaded from the plain type.
    pub fn loaded(value: T) -> Self {
        let f = Self::new();
        f.initialize(value);
        f // dirty remains false because initialize() does not set dirty
    }

    pub fn is_dirty(&self) -> bool {
        self.dirty.load(Ordering::Relaxed)
    }

    /// Clear the dirty flag. Takes &self because AtomicBool allows interior mutability.
    pub fn reset_dirty(&self) {
        self.dirty.store(false, Ordering::Relaxed);
    }
}

impl<T> Default for TrackedField<T> {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tracked_field_tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn new_field_is_uninitialized_and_clean() {
        let f = TrackedField::<String>::new();
        assert!(f.get().is_none());
        assert!(!f.is_dirty());
    }

    #[test]
    fn mutated_is_initialized_and_dirty() {
        let f = TrackedField::mutated("hello".to_string());
        assert_eq!(f.get(), Some(&"hello".to_string()));
        assert!(f.is_dirty());
    }

    #[test]
    fn initialize_sets_value_on_empty_field() {
        let f = TrackedField::new();
        f.initialize("world".to_string());
        assert_eq!(f.get(), Some(&"world".to_string()));
    }

    #[test]
    fn initialize_is_noop_when_already_set() {
        let f = TrackedField::new();
        f.initialize("first".to_string());
        f.initialize("second".to_string()); // must not overwrite
        assert_eq!(f.get(), Some(&"first".to_string()));
    }

    #[test]
    fn initialize_does_not_mark_dirty() {
        let f = TrackedField::new();
        f.initialize("x".to_string());
        assert!(!f.is_dirty(), "load path must not mark dirty");
    }

    #[test]
    fn reset_dirty_clears_flag() {
        let f = TrackedField::mutated(42u32);
        assert!(f.is_dirty());
        f.reset_dirty();
        assert!(!f.is_dirty());
        assert_eq!(f.get(), Some(&42)); // value unchanged
    }

    #[test]
    fn arc_reset_dirty_works_through_shared_ref() {
        let f = Arc::new(TrackedField::mutated("shared".to_string()));
        let f2 = Arc::clone(&f);
        assert!(f.is_dirty());
        f2.reset_dirty(); // reset via the clone
        assert!(!f.is_dirty(), "dirty flag is shared across Arc clones");
    }

    #[test]
    fn cow_pattern_original_unaffected_by_replacement() {
        let original = Arc::new(TrackedField::mutated("old".to_string()));
        let _checkout_copy = Arc::clone(&original);

        let new_field = Arc::new(TrackedField::mutated("new".to_string()));
        assert_eq!(original.get(), Some(&"old".to_string()));
        assert_eq!(new_field.get(), Some(&"new".to_string()));
        assert!(new_field.is_dirty());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pari_macros::Tracked;

    // --- Task 3.2: flat struct derive ---

    #[derive(Tracked)]
    struct FlatStruct {
        pub id: String,
        pub name: String,
    }

    #[test]
    fn derive_flat_struct_from_impl_works() {
        let plain = FlatStruct { id: "x".to_string(), name: "foo".to_string() };
        let tracked = TrackedFlatStruct::from(plain);
        assert_eq!(*tracked.id, "x");
        assert_eq!(*tracked.name, "foo");
    }

    #[test]
    fn derive_flat_struct_all_fields_start_dirty() {
        let plain = FlatStruct { id: "x".to_string(), name: "foo".to_string() };
        let tracked = TrackedFlatStruct::from(plain);
        assert!(tracked.id.is_dirty());
        assert!(tracked.name.is_dirty());
    }

    #[test]
    fn derive_flat_struct_dirty_fields_returns_only_dirty_names() {
        let plain = FlatStruct { id: "x".to_string(), name: "foo".to_string() };
        let mut tracked = TrackedFlatStruct::from(plain);
        tracked.id.reset_dirty();
        tracked.name.reset_dirty();
        *tracked.name = "bar".to_string();
        let dirty = tracked.dirty_fields();
        assert!(!dirty.contains(&"id"));
        assert!(dirty.contains(&"name"));
    }

    // --- Task 3.4: enum derive ---

    #[derive(Tracked)]
    struct DeriveInnerA {
        pub value: String,
    }

    #[derive(Tracked)]
    struct DeriveInnerB {
        pub count: u32,
    }

    #[derive(Tracked)]
    enum DeriveMyEnum {
        A(DeriveInnerA),
        B(DeriveInnerB),
    }

    #[test]
    fn derive_enum_from_variant_a_converts_inner() {
        let plain = DeriveMyEnum::A(DeriveInnerA { value: "hi".to_string() });
        let tracked = TrackedDeriveMyEnum::from(plain);
        match &tracked {
            TrackedDeriveMyEnum::A(a) => assert_eq!(*a.value, "hi"),
            _ => panic!("expected variant A"),
        }
    }

    #[test]
    fn derive_enum_dirty_fields_delegates_to_active_variant() {
        let plain = DeriveMyEnum::A(DeriveInnerA { value: "hi".to_string() });
        let mut tracked = TrackedDeriveMyEnum::from(plain);
        if let TrackedDeriveMyEnum::A(a) = &mut tracked {
            a.value.reset_dirty();
        }
        assert!(tracked.dirty_fields().is_empty());
        if let TrackedDeriveMyEnum::A(a) = &mut tracked {
            *a.value = "bye".to_string();
        }
        assert!(tracked.dirty_fields().contains(&"value"));
    }

    // --- Task 3.6: generic struct derive ---

    #[derive(Tracked)]
    struct DeriveInner {
        pub label: String,
    }

    #[derive(Tracked)]
    struct DeriveGenStruct<S> {
        pub meta: String,
        pub item: S,
    }

    #[test]
    fn derive_generic_struct_from_impl_works() {
        let plain = DeriveGenStruct {
            meta: "test".to_string(),
            item: DeriveInner { label: "hello".to_string() },
        };
        let tracked = TrackedDeriveGenStruct::<TrackedDeriveInner>::from(plain);
        assert_eq!(*tracked.meta, "test");
        assert_eq!(*tracked.item.label, "hello");
    }

    #[test]
    fn derive_generic_struct_dirty_fields_excludes_generic_param() {
        let plain = DeriveGenStruct {
            meta: "test".to_string(),
            item: DeriveInner { label: "hello".to_string() },
        };
        let mut tracked = TrackedDeriveGenStruct::<TrackedDeriveInner>::from(plain);
        tracked.meta.reset_dirty();
        // item (bare generic param) is excluded from dirty_fields
        assert!(tracked.dirty_fields().is_empty());
        *tracked.meta = "changed".to_string();
        assert!(tracked.dirty_fields().contains(&"meta"));
    }

    // --- Task 3.8: #[tracked(map_key)] derive ---

    #[derive(Tracked)]
    struct MapStep {
        pub id: String,
        pub desc: String,
    }

    impl HasId for MapStep {
        fn id(&self) -> &str {
            &self.id
        }
    }

    #[derive(Tracked)]
    struct DeriveContainer<S>
    where
        S: HasId,
    {
        pub name: String,
        #[tracked(map_key = "id")]
        pub items: Vec<S>,
    }

    #[test]
    fn derive_map_key_field_becomes_tracked_map() {
        let plain = DeriveContainer {
            name: "x".to_string(),
            items: vec![
                MapStep { id: "a".to_string(), desc: "first".to_string() },
                MapStep { id: "b".to_string(), desc: "second".to_string() },
            ],
        };
        let tracked = TrackedDeriveContainer::<TrackedMapStep>::from(plain);
        assert_eq!(tracked.items.len(), 2);
        assert!(tracked.items.inserted.contains_key("a"));
    }

    #[test]
    fn derive_map_key_dirty_fields_includes_field_when_changed() {
        let plain = DeriveContainer {
            name: "x".to_string(),
            items: vec![MapStep { id: "a".to_string(), desc: "d".to_string() }],
        };
        let mut tracked = TrackedDeriveContainer::<TrackedMapStep>::from(plain);
        tracked.name.reset_dirty();
        // items.has_changes() is true (inserted), so "items" appears in dirty_fields
        assert!(tracked.dirty_fields().contains(&"items"));
    }

    // --- Tracked<T> ---

    #[test]
    fn tracked_new_starts_dirty() {
        let t = Tracked::new("hello".to_string());
        assert!(t.is_dirty());
    }

    #[test]
    fn tracked_deref_reads_transparently() {
        let t = Tracked::new("hello".to_string());
        assert_eq!(*t, "hello");
        assert_eq!(t.len(), 5);
    }

    #[test]
    fn tracked_deref_mut_marks_dirty() {
        let mut t = Tracked::new("hello".to_string());
        t.reset_dirty();
        assert!(!t.is_dirty());
        t.push_str(" world");
        assert!(t.is_dirty());
    }

    #[test]
    fn tracked_reset_clears_dirty_flag() {
        let mut t = Tracked::new("hello".to_string());
        assert!(t.is_dirty());
        t.reset_dirty();
        assert!(!t.is_dirty());
        assert_eq!(*t, "hello"); // value unchanged
    }

    #[test]
    fn tracked_mutation_after_reset_re_dirties() {
        let mut t = Tracked::new("hello".to_string());
        t.reset_dirty();
        *t = "world".to_string();
        assert!(t.is_dirty());
    }

    // --- TrackedMap: basic insert / get / remove ---

    #[test]
    fn tracked_map_insert_records_key_in_inserted_set() {
        let mut m: TrackedMap<String, u32> = TrackedMap::new();
        m.insert("eng-lead".to_string(), 1);
        assert!(m.inserted.contains_key("eng-lead"));
        assert_eq!(m.get(&"eng-lead".to_string()), Some(&1));
    }

    #[test]
    fn tracked_map_remove_retains_full_value_in_removed_map() {
        let mut m: TrackedMap<String, u32> = TrackedMap::new();
        m.insert("eng-lead".to_string(), 42);
        m.reset_tracked();
        m.remove(&"eng-lead".to_string());
        assert!(m.get(&"eng-lead".to_string()).is_none());
        assert_eq!(m.removed.get("eng-lead"), Some(&42));
    }

    #[test]
    fn tracked_map_get_mut_records_key_in_modified_set() {
        let mut m: TrackedMap<String, u32> = TrackedMap::new();
        m.insert("eng-lead".to_string(), 1);
        m.reset_tracked(); // clear inserted
        m.get_mut(&"eng-lead".to_string());
        assert!(m.modified.contains_key("eng-lead"));
    }

    #[test]
    fn tracked_map_inserted_takes_precedence_over_modified() {
        let mut m: TrackedMap<String, u32> = TrackedMap::new();
        m.insert("a".to_string(), 1);
        m.get_mut(&"a".to_string()); // would add to modified
        assert!(m.inserted.contains_key("a"));
        assert!(!m.modified.contains_key("a"), "inserted should take precedence");
    }

    #[test]
    fn tracked_map_insertion_order_preserved() {
        let mut m: TrackedMap<String, u32> = TrackedMap::new();
        m.insert("a".to_string(), 1);
        m.insert("b".to_string(), 2);
        m.insert("c".to_string(), 3);
        let keys: Vec<&String> = m.keys().collect();
        assert_eq!(keys, vec!["a", "b", "c"]);
    }

    #[test]
    fn tracked_map_shift_remove_preserves_order_of_remaining() {
        let mut m: TrackedMap<String, u32> = TrackedMap::new();
        m.insert("a".to_string(), 1);
        m.insert("b".to_string(), 2);
        m.insert("c".to_string(), 3);
        m.reset_tracked();
        m.remove(&"b".to_string());
        let keys: Vec<&String> = m.keys().collect();
        assert_eq!(keys, vec!["a", "c"]);
    }

    // --- has_changes / reset_tracked ---

    #[test]
    fn tracked_map_has_changes_when_inserted() {
        let mut m: TrackedMap<String, u32> = TrackedMap::new();
        m.insert("a".to_string(), 1);
        assert!(m.has_changes());
    }

    #[test]
    fn tracked_map_has_changes_when_removed() {
        let mut m: TrackedMap<String, u32> = TrackedMap::new();
        m.insert("a".to_string(), 1);
        m.reset_tracked();
        m.remove(&"a".to_string());
        assert!(m.has_changes());
    }

    #[test]
    fn tracked_map_reset_tracked_clears_all_state() {
        let mut m: TrackedMap<String, u32> = TrackedMap::new();
        m.insert("a".to_string(), 1);
        m.insert("c".to_string(), 3);
        m.reset_tracked();
        m.remove(&"c".to_string());
        m.reset_tracked();
        assert!(!m.has_changes());
        assert!(m.removed.is_empty());
    }

    // --- from_vec ---

    struct TestItem {
        id: String,
        val: u32,
    }
    impl HasId for TestItem {
        fn id(&self) -> &str {
            &self.id
        }
    }

    #[test]
    fn tracked_map_from_vec_preserves_order_and_marks_inserted() {
        let items = vec![
            TestItem { id: "a".to_string(), val: 1 },
            TestItem { id: "b".to_string(), val: 2 },
        ];
        let m: TrackedMap<String, u32> = TrackedMap::from_vec(items, |item| item.val);
        let keys: Vec<&String> = m.keys().collect();
        assert_eq!(keys, vec!["a", "b"]);
        assert!(m.inserted.contains_key("a"));
        assert!(m.inserted.contains_key("b"));
    }
}
