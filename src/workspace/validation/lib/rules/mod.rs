//! Per-entity validation schemas and the shared rule primitives they
//! compose. Each entity file (`role.rs`, `workflow.rs`, …) defines a
//! `*_validation_schema()` builder; the `structural`, `semantic`, and
//! `cross_entity` submodules hold the reusable primitives those
//! builders call into.

pub mod artifact_kind;
pub mod cross_entity;
pub mod hook;
pub mod relay;
pub mod role;
pub mod semantic;
pub mod structural;
pub mod task;
pub mod team;
pub mod workflow;
