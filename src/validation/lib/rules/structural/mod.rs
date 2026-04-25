//! Structural rule primitives — sync functions that inspect a single
//! field value and return `Vec<PrimitiveError>`. Entity schemas wrap
//! these into field-level `AnyStructuralRule` closures.

pub mod hook;
pub mod primitives;
pub mod raci;
pub mod relay;
pub mod role;
pub mod task;
pub mod team;
pub mod workflow;
