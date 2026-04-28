//! Functional integration tests, one file per user job.
//!
//! Files are added as user jobs are covered. See the order in
//! [TODO.md](../../TODO.md) Phase 1.

#[path = "author_role.rs"]
pub mod author_role;

#[path = "author_team.rs"]
pub mod author_team;

#[path = "author_workflow.rs"]
pub mod author_workflow;

#[path = "modify_persisted_entity.rs"]
pub mod modify_persisted_entity;

#[path = "author_workflow_with_intercepts.rs"]
pub mod author_workflow_with_intercepts;

#[path = "author_embedded_workflow.rs"]
pub mod author_embedded_workflow;

#[path = "author_reusable_workflow.rs"]
pub mod author_reusable_workflow;

#[path = "author_relay.rs"]
pub mod author_relay;

#[path = "validation_failures.rs"]
pub mod validation_failures;

#[path = "lifecycle_failures.rs"]
pub mod lifecycle_failures;

#[path = "validation_timing.rs"]
pub mod validation_timing;
