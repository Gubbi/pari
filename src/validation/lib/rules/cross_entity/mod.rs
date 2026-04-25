//! Cross-entity rules — async rules that query the store via
//! `EntityClient::has_ref`. `common.rs` holds the shared
//! `check_refs` primitive and the `ref_check_rule!` macro used by
//! entity schemas; the per-entity files define rules that need more
//! than a plain ref-existence check.

pub mod common;
pub mod intercepts;
pub mod relay;
pub mod team;
pub mod workflow;
