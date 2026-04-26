//! Substrate parameterization for functional tests.
//!
//! Scenarios where the substrate is incidental to the behavior under
//! test run against both shipped backends via `run_with`. Substrate-
//! specific scenarios pin to a single backend directly.
//!
//! Drive the parameterization with `rstest`:
//!
//! ```ignore
//! #[rstest]
//! #[case::in_memory(SubstrateKind::InMemory)]
//! #[case::repo(SubstrateKind::Repo)]
//! #[tokio::test]
//! async fn scenario(#[case] kind: SubstrateKind) {
//!     run_with(kind, || async {
//!         // ... scenario body ...
//!     }).await;
//! }
//! ```

use std::future::Future;

use pari::substrate::{InMemorySubstrate, RepoSubstrate};
use tempfile::TempDir;

/// Which backend a scenario should run against.
#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub enum SubstrateKind {
    InMemory,
    Repo,
}

/// Run `scenario` against the substrate identified by `kind`.
///
/// For `Repo`, a fresh tempdir is allocated for the scenario and
/// dropped after it completes.
#[allow(dead_code)]
pub async fn run_with<F, Fut>(kind: SubstrateKind, scenario: F)
where
    F: FnOnce() -> Fut,
    Fut: Future<Output = ()>,
{
    match kind {
        SubstrateKind::InMemory => {
            pari::with(InMemorySubstrate::new(), scenario).await;
        }
        SubstrateKind::Repo => {
            let dir = TempDir::new().expect("create tempdir for RepoSubstrate");
            let substrate =
                RepoSubstrate::new(dir.path().to_path_buf()).expect("construct RepoSubstrate");
            pari::with(substrate, scenario).await;
            drop(dir);
        }
    }
}
