use crate::entity::{AnyEntityRef, StoreEntity};

/// A single tracked-entity change emitted by the store for persistence.
pub enum EntityChange<'a> {
    Added(&'a StoreEntity),
    Modified(&'a StoreEntity, &'a [&'a str]),
    Removed(&'a AnyEntityRef),
}
