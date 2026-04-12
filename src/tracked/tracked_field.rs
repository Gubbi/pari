use std::{
    sync::OnceLock,
    sync::atomic::{AtomicBool, Ordering},
};

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
        f
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

#[cfg(test)]
mod tests {
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
        f.initialize("second".to_string());
        assert_eq!(f.get(), Some(&"first".to_string()));
    }

    #[test]
    fn initialize_does_not_mark_dirty() {
        let f = TrackedField::new();
        f.initialize("x".to_string());
        assert!(!f.is_dirty());
    }

    #[test]
    fn loaded_is_initialized_and_clean() {
        let f = TrackedField::loaded("x".to_string());
        assert_eq!(f.get(), Some(&"x".to_string()));
        assert!(!f.is_dirty());
    }

    #[test]
    fn reset_dirty_clears_flag() {
        let f = TrackedField::mutated(42u32);
        assert!(f.is_dirty());
        f.reset_dirty();
        assert!(!f.is_dirty());
        assert_eq!(f.get(), Some(&42));
    }

    #[test]
    fn arc_reset_dirty_works_through_shared_ref() {
        let f = Arc::new(TrackedField::mutated("shared".to_string()));
        let f2 = Arc::clone(&f);
        assert!(f.is_dirty());
        f2.reset_dirty();
        assert!(!f.is_dirty());
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
