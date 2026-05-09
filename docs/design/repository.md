# Repository Organization

How the Pari codebase is organized as a Cargo workspace, what gets published where, and how versions move together. This document is the source of truth for repo-level decisions; [docs/conventions.md](../conventions.md) indexes them.

This is not a C4 design level — it sits cross-cutting alongside [test.md](./test.md). For the architectural layout of the system itself, see [README.md](./README.md) and [framework.md](./framework.md).

## Workspace Shape

Pari is a Cargo workspace. Members today:

| Member | Crate kind | Role |
|---|---|---|
| `.` (root) | Library | `pari-core` — the runtime library |
| `pari-macros/` | Proc-macro | Derive macros consumed by `pari-core` |
| `xtask/` | Binary | Internal build / codegen tooling |

Future members (deferred until they exist):

| Member | Crate kind | Role |
|---|---|---|
| `pari-cli/` | Binary | End-user CLI |
| `pari-mcp/` | Binary | Deployable MCP server |
| `pari-local-mcp/` | Binary | Locally-installed MCP server |

## Library Name vs Package Name

The root crate's **package name** (its identity on crates.io) is `pari-core`. Its **library name** (what downstream code writes after `use`) is `pari`.

```toml
[package]
name = "pari-core"

[lib]
name = "pari"
```

Downstream code reads `use pari::workspace::...;` while the dependency line is `pari-core = "x.y.z"`. This leaves room for a future top-level umbrella crate without forcing a global rename.

## Distribution Targets

Each crate has exactly one distribution channel.

| Crate | Channel | Mechanism |
|---|---|---|
| `pari-core` | crates.io | `cargo publish` |
| `pari-macros` | crates.io | `cargo publish` (technically internal — used via `pari-core` re-exports) |
| `xtask` | None — repo-internal only | `publish = false` |
| `pari-cli` | GitHub Releases + package managers | `cargo-dist` (Homebrew, installer scripts, `cargo binstall`); also `cargo install` from crates.io as fallback |
| `pari-local-mcp` | GitHub Releases + package managers | Same `cargo-dist` pipeline as `pari-cli` |
| `pari-mcp` | Docker image and/or npm shim | Decision deferred; `cargo-dist` may produce the npm shim, Docker built separately |

`pari-macros` follows the standard `serde` / `serde_derive` convention: technically published so `pari-core` can resolve it from the registry, but documented as internal — users consume macros via `pari-core` re-exports, not by adding `pari-macros` to their `Cargo.toml`.

## Versioning Policy

`pari-core` is the **lockstep root**. When `pari-core` bumps, every crate that depends on it bumps too — `pari-macros`, `pari-cli`, `pari-mcp`, `pari-local-mcp`. This guarantees a single coherent release of the library and its consumers.

Binary crates that **don't depend on each other** version independently. A `pari-cli`-only fix bumps `pari-cli` alone; `pari-mcp` and `pari-local-mcp` are unaffected. The lockstep rule cascades downward from `pari-core`, not sideways across binaries.

Concretely:

- `pari-core` bump → bumps `pari-macros`, `pari-cli`, `pari-mcp`, `pari-local-mcp`.
- `pari-cli` bump → bumps only `pari-cli`.
- `pari-mcp` bump → bumps only `pari-mcp`.
- `pari-local-mcp` bump → bumps only `pari-local-mcp`.

## Stable Rust Requirement

The workspace builds on stable Rust. No `#![feature(...)]` gates, no nightly-only APIs.

The reason is publishability: a crate that requires nightly can be published, but most downstream users cannot depend on it without switching their own project to nightly — a non-starter for a library released as `pari-core`.

This is a hard requirement. Any feature that requires nightly is either (a) replaced with a stable equivalent, (b) deferred until the feature stabilizes, or (c) brings the team back to revisit this rule. It is not silently re-enabled.

## Inter-Workspace Dependency Handling

Workspace-internal crate dependencies use both `path` and `version`:

```toml
pari-macros = { path = "pari-macros", version = "0.1" }
```

The `path` is what Cargo uses inside the workspace; the `version` is what Cargo resolves from crates.io after publish. Without a `version`, `cargo publish` rejects the dependent crate.

For path-only consumers within the workspace that should never publish (e.g. `xtask`), use Cargo's package rename to keep import paths stable across the rename:

```toml
pari = { package = "pari-core", path = "..", features = ["..."] }
```

Common metadata (`edition`, `license`, frequently shared deps) consolidates under `[workspace.package]` and `[workspace.dependencies]` in the root manifest once the workspace grows past 4–5 members. Members then write `version.workspace = true` etc.

## Release Tooling

- **Library crates** (`pari-core`, `pari-macros`): published with `cargo publish -p <name>`. Order: leaves first (`pari-macros` before `pari-core`).
- **Binary crates** (`pari-cli`, `pari-local-mcp`): released via [`cargo-dist`](https://github.com/axodotdev/cargo-dist) — generates a GitHub Actions workflow that on tag-push cross-compiles for Linux/macOS/Windows, builds installer scripts, uploads to GitHub Releases, and optionally publishes to a Homebrew tap.
- **`pari-mcp`**: distribution mechanism deferred. Likely Docker for deployable use, possibly an npm shim for `npx`-style invocation.

A single release tag (`v0.x.y`) covers the lockstep release of `pari-core` + dependents. Independent binary releases use their own tag namespace (e.g. `pari-cli-v0.x.y`).
