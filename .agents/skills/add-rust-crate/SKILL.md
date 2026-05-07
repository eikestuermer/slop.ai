---
name: add-rust-crate
description: >-
  Scaffold a new Rust crate in the slop.ai workspace under crates/. Use when
  the user asks to "add a new crate", "scaffold slop-X", or "create a new
  Rust library for X". Produces a Cargo.toml that uses the workspace's
  shared dependencies, a stub lib.rs with a unit test, registers the crate
  in the root Cargo.toml's workspace.members + workspace.dependencies, and
  optionally adds a per-crate AGENTS.md when the crate carries
  domain-specific conventions.
---

# Add a Rust crate to the slop.ai workspace

Use this skill when the user asks for a new Rust crate. The shape is
prescribed by the existing crates; consistency matters because the agent
guidance, CI, and `cargo deny` config all assume it.

## Inputs to gather

- **Name**: `slop-<kebab>` (we always prefix with `slop-`).
- **One-line description**: this becomes the `description` field in the new `Cargo.toml`.
- **Whether the crate is a library** (default) **or a binary** (apps go under `apps/`, not `crates/`).
- **Native deps?** ffmpeg subprocess only, or does it need `whisper-cpp`/`ort`/CMake? Native deps go behind a feature flag, off by default.

## Workflow

1. Create `crates/<name>/Cargo.toml`:

   ```toml
   [package]
   name = "<name>"
   version.workspace = true
   edition.workspace = true
   license.workspace = true
   description = "<one-line description>"

   [dependencies]
   slop-core = { workspace = true }    # if you need timeline types
   serde = { workspace = true }
   serde_json = { workspace = true }
   thiserror = { workspace = true }    # libraries only

   [dev-dependencies]
   ```

   Use `workspace = true` for every shared dependency listed in the root [Cargo.toml](Cargo.toml)'s `[workspace.dependencies]`. Do not pin a version locally if the workspace already defines one.

2. Create `crates/<name>/src/lib.rs`:

   ```rust
   //! # <name>
   //!
   //! <one-paragraph description: what this crate does, who uses it, and
   //! what its place in the architecture is.>

   #![deny(missing_docs)]

   pub mod error;

   pub use error::{Error, Result};

   #[cfg(test)]
   mod tests {
       use super::*;

       #[test]
       fn smoke() {
           // At least one unit test on day one. The CI gate counts every
           // test in the workspace.
       }
   }
   ```

3. Create `crates/<name>/src/error.rs` using `thiserror`:

   ```rust
   use thiserror::Error;

   pub type Result<T, E = Error> = std::result::Result<T, E>;

   #[derive(Debug, Error)]
   pub enum Error {
       #[error("io: {0}")]
       Io(#[from] std::io::Error),
   }
   ```

4. Update root [Cargo.toml](Cargo.toml):
   - Add `"crates/<name>"` to `workspace.members`.
   - Add `<name> = { path = "crates/<name>", version = "0.1" }` to `workspace.dependencies`. **Always include the `version` field**: cargo-deny rejects path-only deps as wildcards.

5. Run `cargo check -p <name>` to verify it compiles.

6. If the new crate has its own architectural rules (e.g. `slop-color` with its CDL discipline), add a `crates/<name>/AGENTS.md` with the per-crate rules.

## When to add the crate as a workspace dep elsewhere

- If a new app or another crate needs the new crate, add `<name> = { workspace = true }` to *its* `Cargo.toml`. Do not use a relative `path = "../../crates/<name>"`; that's another wildcard violation.

## When to skip this skill

- If you're adding a binary (`slop-cli`, `slop-sync-server`), it goes under [apps/](apps/) not `crates/`, with its own `[[bin]]` target. The Cargo.toml shape is the same but you also add `default-run = "<bin-name>"` if multiple binaries exist.
- If you need PyO3 / napi-rs cdylib, that's [bindings/python](bindings/python) or [bindings/node](bindings/node), not `crates/`. The macOS link-flag config in [.cargo/config.toml](.cargo/config.toml) is required.

## Verify before handing off

- `cargo check --workspace` passes.
- `cargo test -p <name>` passes (the smoke test).
- `cargo clippy -p <name> --all-targets -- -D warnings` passes.
- `cargo deny check` passes (in particular: no wildcards, no rejected licenses).
- The new crate appears in `cargo metadata --no-deps --format-version 1 | jq '.workspace_members'`.
