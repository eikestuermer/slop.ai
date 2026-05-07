//! Plugin manifest schema.

use serde::{Deserialize, Serialize};

/// Required capability granted to a plugin.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum Capability {
    /// Register one or more effects under the listed `kind` strings.
    Effects {
        /// Effect kinds the plugin will register.
        kinds: Vec<String>,
    },
    /// Register one or more scoring features.
    ScoringFeatures {
        /// Feature names.
        names: Vec<String>,
    },
    /// Register one or more exporters.
    Exporters {
        /// Exporter format ids (e.g. `"avid-aaf"`, `"edl-cmx3600"`).
        formats: Vec<String>,
    },
    /// Register one or more prompt-pack styles.
    PromptPackStyles {
        /// Style ids.
        styles: Vec<String>,
    },
    /// Read project files (off by default).
    FsRead,
    /// Write project files (off by default).
    FsWrite,
    /// Outbound network access (off by default).
    Network {
        /// Allowed hosts.
        allow_hosts: Vec<String>,
    },
}

/// Manifest fields.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    /// Reverse-DNS plugin id.
    pub id: String,
    /// Semver version.
    pub version: String,
    /// Display name.
    pub name: String,
    /// Author name + email.
    pub author: String,
    /// SPDX license identifier.
    pub license: String,
    /// Source URL.
    pub repository: String,
    /// What ABI version the wasm targets.
    pub abi_version: u32,
    /// Capabilities the plugin requests.
    pub capabilities: Vec<Capability>,
    /// SHA-256 of the .wasm file (hex).
    pub wasm_sha256: String,
    /// Sigstore identity claim used to sign (e.g. GitHub Actions OIDC `iss`).
    pub signing_identity: Option<String>,
}

impl PluginManifest {
    /// The current Slop AI plugin ABI version. Bumped on breaking changes
    /// to the `.wit` interface.
    pub const CURRENT_ABI: u32 = 1;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capability_round_trips() {
        let c = Capability::Effects {
            kinds: vec!["color-space".into()],
        };
        let s = serde_json::to_string(&c).unwrap();
        let back: Capability = serde_json::from_str(&s).unwrap();
        assert_eq!(c, back);
    }
}
