# Task 01 — TrackedField<T>

## Scope

Implement `TrackedField<T>`: the OnceLock-backed field primitive used by all tracked entities. All fields on tracked entity structs are `Arc<TrackedField<T>>`. This is the foundational building block; everything else depends on it.

Rewrite `src/tracked.rs`. Keep `TrackedMap<K,V>` and `HasId` (they are still used for step collections). Add `TrackedField<T>`.

## Files

- `src/tracked.rs` — add `TrackedField<T>`, keep `TrackedMap<K,V>` and `HasId`

## No New Dependencies

Uses only `std::sync::{OnceLock, Arc}` and `std::sync::atomic::{AtomicBool, Ordering}`.

## Types and Signatures

```rust
use std::sync::{Arc, OnceLock};
use std::sync::atomic::{AtomicBool, Ordering};

/// OnceLock-backed field with an atomic dirty flag.
///
/// All domain fields on tracked entities are `Arc<TrackedField<T>>`.
/// The Arc enables cheap clone at checkout and COW replacement at mutation.
///
/// Two write paths:
/// - `initialize()` — write-once, used by the load path and deserializer.
///   Silently ignores if the field is already initialized.
/// - COW replacement — setters create a new `TrackedField::with_value(v)`
///   and swap the Arc pointer on the tracked entity. The old Arc is untouched.
pub struct TrackedField<T> {
    value: OnceLock<T>,
    dirty: AtomicBool,
}

impl<T> TrackedField<T> {
    /// Create an empty, clean field (uninitialized, not dirty).
    pub fn new() -> Self;

    /// Create a field pre-seeded with `value` and marked dirty.
    /// Used by setters to build the replacement Arc before swapping.
    pub fn with_value(value: T) -> Self;

    /// Write-once initialization. Used by the load path and deserializer.
    /// If the field is already initialized, this is a no-op.
    pub fn initialize(&self, value: T);

    /// Returns the value if initialized, None otherwise.
    pub fn get(&self) -> Option<&T>;

    pub fn is_dirty(&self) -> bool;

    /// Clear the dirty flag. Takes &self because AtomicBool allows interior mutability.
    pub fn reset_dirty(&self);
}

impl<T> Default for TrackedField<T> {
    fn default() -> Self { Self::new() }
}
```

## TDD: Tests to Write First

Add a `#[cfg(test)]` module to `src/tracked.rs`:

```rust
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
    fn with_value_is_initialized_and_dirty() {
        let f = TrackedField::with_value("hello".to_string());
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
        let f = TrackedField::with_value(42u32);
        assert!(f.is_dirty());
        f.reset_dirty();
        assert!(!f.is_dirty());
        assert_eq!(f.get(), Some(&42)); // value unchanged
    }

    #[test]
    fn arc_reset_dirty_works_through_shared_ref() {
        let f = Arc::new(TrackedField::with_value("shared".to_string()));
        let f2 = Arc::clone(&f);
        assert!(f.is_dirty());
        f2.reset_dirty(); // reset via the clone
        assert!(!f.is_dirty(), "dirty flag is shared across Arc clones");
    }

    #[test]
    fn cow_pattern_original_unaffected_by_replacement() {
        // Simulate: checkout gives clone of Arc, setter creates new Arc
        let original = Arc::new(TrackedField::with_value("old".to_string()));
        let _checkout_copy = Arc::clone(&original); // simulates checkout

        // Setter creates a brand-new TrackedField and swaps the Arc
        let new_field = Arc::new(TrackedField::with_value("new".to_string()));
        // The tracked entity's field pointer would be replaced with new_field.
        // The original Arc (held by checkout_copy and the store) is untouched.
        assert_eq!(original.get(), Some(&"old".to_string()));
        assert_eq!(new_field.get(), Some(&"new".to_string()));
        assert!(new_field.is_dirty());
    }
}
```

## Implementation Notes

- `OnceLock::set` returns `Err(value)` if already initialized; use `let _ = self.value.set(value)` to discard the error in `initialize()`.
- `AtomicBool` with `Ordering::Relaxed` is sufficient for the dirty flag (it is never used as a synchronization fence).
- `with_value` constructs a new OnceLock and calls `let _ = inner.set(value)` before returning. This always succeeds on a fresh OnceLock.
- Do NOT implement `DerefMut` on `TrackedField` — mutation is exclusively via Arc replacement, never via mutable deref.

## Acceptance Criteria

All tests in `tracked_field_tests` pass. `cargo test tracked_field` green.
