//! Decentralized plugin registry.
//!
//! Slop AI's marketplace is intentionally not a centralized service: a
//! registry is a static-site repo that lists plugins as `(id, git_url,
//! manifest_url, sigstore_bundle_url)` tuples. Anyone can run their own
//! registry; the desktop app supports adding multiple registry URLs.
//!
//! This struct manages the local index cache.

use crate::manifest::PluginManifest;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// One registry entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryEntry {
    /// Plugin id.
    pub id: String,
    /// Latest version.
    pub version: String,
    /// Git URL.
    pub repository: String,
    /// URL of the manifest TOML.
    pub manifest_url: String,
    /// URL of the .wasm file.
    pub wasm_url: String,
    /// URL of the Sigstore bundle.
    pub sigstore_bundle_url: Option<String>,
}

/// Local cache of installed + indexed plugins.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PluginRegistry {
    /// Registry root URLs the user has subscribed to.
    pub registries: Vec<String>,
    /// Cached entries, keyed by `(registry_url, id)`.
    pub index: Vec<RegistryEntry>,
    /// Installed manifests + paths.
    pub installed: Vec<InstalledPlugin>,
}

/// One installed plugin.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstalledPlugin {
    /// Manifest.
    pub manifest: PluginManifest,
    /// Local path to the .wasm.
    pub wasm_path: PathBuf,
    /// Local path to the Sigstore bundle (if any).
    pub sigstore_bundle: Option<PathBuf>,
    /// User-granted capabilities.
    pub granted: Vec<crate::Capability>,
}

impl PluginRegistry {
    /// Add a registry root URL.
    pub fn subscribe(&mut self, url: impl Into<String>) {
        let url = url.into();
        if !self.registries.contains(&url) {
            self.registries.push(url);
        }
    }
}
