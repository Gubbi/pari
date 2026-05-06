//! Shared helpers for integration tests.
//!
//! `substrate` is currently disabled — its `run_with` harness is built
//! around the pre-refactor `pari::with` entry point and will be
//! rewritten alongside the functional tests in the dedicated
//! test-refactor thread.

#[cfg(any())]
pub mod substrate;
