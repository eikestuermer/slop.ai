//! # slop-plugin
//!
//! WASI-Component-Model plugin host for Slop AI.
//!
//! ## Why WASI 0.2 Component Model
//!
//! WASM Components are the SOTA sandbox shape (2024+): typed interfaces
//! across language boundaries, capability-based security, deterministic
//! execution, signed bytecode. Plugins author against a published `.wit`
//! interface; the host enforces the capability surface.
//!
//! ## Plugin contract
//!
//! Each plugin ships:
//! - `slop-plugin.wasm` — the component bytecode.
//! - `slop-plugin.toml` — the manifest (id, version, author, capabilities,
//!   signature).
//! - `slop-plugin.sig` — Sigstore detached signature over the wasm + manifest.
//!
//! ## Capabilities surface
//!
//! V3.0 plugins can register:
//! - **Effects** — new `EffectNode.kind` values backed by an exported
//!   `effect-process` function.
//! - **Scoring features** — new candidate-moment features.
//! - **Exporters** — custom output formats.
//! - **Prompt-pack styles** — new planner prompt scaffolds.
//!
//! Plugins cannot read project files, network, or env unless the manifest
//! declares those capabilities and the host grants them at install time.

#![deny(missing_docs)]

pub mod host;
pub mod manifest;
pub mod registry;
pub mod signature;

pub use host::{PluginHost, PluginHostError};
pub use manifest::{Capability, PluginManifest};
pub use registry::PluginRegistry;
pub use signature::{verify_plugin, SignatureError};
