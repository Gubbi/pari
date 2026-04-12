//! Change tracking primitives aligned with the current entity design.
//!
//! [`TrackedField<T>`] is the only remaining tracking primitive. Tracked
//! entities store domain fields as `Arc<TrackedField<T>>`, allowing:
//! - write-once initialization during load/deserialization
//! - cheap clone at checkout
//! - COW replacement during mutation

mod tracked_field;

pub use tracked_field::TrackedField;
