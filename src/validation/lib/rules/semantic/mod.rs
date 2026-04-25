//! Entity-local semantic rules — async functions that inspect the
//! tracked entity and may transitively access sibling fields via
//! transparent load, but never query the store.

pub mod workflow;
