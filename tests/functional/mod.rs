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
