//! [`TrackedField<T>`] — the only change-tracking primitive in the layer.

use std::sync::{
    atomic::{AtomicBool, Ordering},
    OnceLock,
};

/// A single trackable field: a write-once value plus a dirty flag.
///
/// Every domain field on a tracked entity is an `Arc<TrackedField<T>>`. The
/// `Arc` makes checkout-time clone cheap and mutation copy-on-write: setters
/// do not mutate the existing field, they build a new
/// [`TrackedField::mutated`] and swap the `Arc`. Previous clones held by
/// in-flight readers keep observing the old value; the dirty flag on the
/// replacement tells the store which fields still need to persist.
///
/// The four constructors model the four ways a field comes into being:
/// [`new`](Self::new) for an empty slot filled later by
/// [`initialize`](Self::initialize) on the load path;
/// [`loaded`](Self::loaded) for a clean field built from a plain entity;
/// [`mutated`](Self::mutated) for the setter-side replacement carrying the
/// dirty flag.
///
/// The dirty flag is `AtomicBool`, not `bool` or `Cell<bool>`. The store
/// and any clients that already hold a clone of the `Arc` share a single
/// allocation; after `substrate.persist` succeeds, the store needs to
/// reset every modified field's dirty flag in-place so all sharers see
/// the field clean without anyone replacing the `Arc`. `AtomicBool`'s
/// interior mutability via `&self` lets [`reset_dirty`](Self::reset_dirty)
/// do that without forcing the whole field type onto a different
/// concurrency primitive.
pub struct TrackedField<T> {
    value: OnceLock<T>,
    dirty: AtomicBool,
}

impl<T> TrackedField<T> {
    /// Create an empty, clean field (uninitialized, not dirty).
    pub fn new() -> Self {
        Self {
            value: OnceLock::new(),
            dirty: AtomicBool::new(false),
        }
    }

    /// Create a field pre-seeded with `value` and marked dirty.
    /// Used by setters to build the replacement Arc before swapping.
    pub fn mutated(value: T) -> Self {
        let lock = OnceLock::new();
        let _ = lock.set(value);
        Self {
            value: lock,
            dirty: AtomicBool::new(true),
        }
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
