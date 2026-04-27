//! Per-entity fixtures.
//!
//! One file per entity kind. Named constructor functions for canonical
//! sample data only — no builders, no assertion helpers, no setup
//! orchestration. Files are added as user jobs require them.

#[path = "role.rs"]
pub mod role;

#[path = "team.rs"]
pub mod team;

#[path = "artifact_kind.rs"]
pub mod artifact_kind;

#[path = "task.rs"]
pub mod task;

#[path = "workflow.rs"]
pub mod workflow;
