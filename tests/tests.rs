//! Integration test binary.
//!
//! A single binary declares every integration test as a submodule so
//! link cost is paid once. Functional tests live under
//! `tests/functional/<user_job>.rs`, fixtures under
//! `tests/fixtures/<entity>.rs`, and shared helpers under
//! `tests/common/`. See [docs/design/test.md](../docs/design/test.md).

#[path = "common/mod.rs"]
mod common;

#[path = "fixtures/mod.rs"]
mod fixtures;

#[path = "functional/mod.rs"]
mod functional;
