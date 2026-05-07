//! Plugin host: load + invoke WASI components.

use crate::manifest::{Capability, PluginManifest};
use std::path::Path;
use thiserror::Error;
use wasmtime::component::{Component, Linker, ResourceTable};
use wasmtime::{Config, Engine, Store};
use wasmtime_wasi::{WasiCtx, WasiCtxBuilder, WasiCtxView, WasiView};

/// Per-plugin runtime context.
pub struct PluginCtx {
    /// WASI capability set granted to this plugin.
    pub wasi: WasiCtx,
    /// Resource table.
    pub table: ResourceTable,
}

impl WasiView for PluginCtx {
    fn ctx(&mut self) -> WasiCtxView<'_> {
        WasiCtxView {
            ctx: &mut self.wasi,
            table: &mut self.table,
        }
    }
}

/// Host errors.
#[derive(Debug, Error)]
pub enum PluginHostError {
    /// Wasmtime engine error.
    #[error("wasmtime: {0}")]
    Engine(String),
    /// Manifest mismatch (e.g. ABI version).
    #[error("abi mismatch: plugin expects v{expected}, host is v{actual}")]
    AbiMismatch {
        /// Plugin's expected ABI version.
        expected: u32,
        /// Host's ABI version.
        actual: u32,
    },
    /// Capability not granted by user.
    #[error("capability not granted: {0:?}")]
    CapabilityRefused(Capability),
    /// Generic.
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

/// Plugin host.
pub struct PluginHost {
    engine: Engine,
}

impl PluginHost {
    /// Construct a host. Single instance per app process.
    pub fn new() -> Result<Self, PluginHostError> {
        let mut cfg = Config::new();
        cfg.wasm_component_model(true);
        cfg.async_support(true);
        let engine = Engine::new(&cfg).map_err(|e| PluginHostError::Engine(e.to_string()))?;
        Ok(Self { engine })
    }

    /// Load a plugin component, validate its manifest against host policy,
    /// build a Linker, and return a Store ready to instantiate.
    pub fn load(
        &self,
        manifest: &PluginManifest,
        wasm_path: &Path,
        granted: &[Capability],
    ) -> Result<(Store<PluginCtx>, Component), PluginHostError> {
        if manifest.abi_version != PluginManifest::CURRENT_ABI {
            return Err(PluginHostError::AbiMismatch {
                expected: manifest.abi_version,
                actual: PluginManifest::CURRENT_ABI,
            });
        }
        // Verify every requested capability has been granted.
        for cap in &manifest.capabilities {
            if !granted.iter().any(|g| caps_match(g, cap)) {
                return Err(PluginHostError::CapabilityRefused(cap.clone()));
            }
        }

        let component = Component::from_file(&self.engine, wasm_path)
            .map_err(|e| PluginHostError::Engine(e.to_string()))?;
        let mut wasi_builder = WasiCtxBuilder::new();
        // By default plugins get *no* filesystem and *no* network. Granting
        // happens by walking the granted capabilities list and toggling
        // bits on `wasi_builder` accordingly.
        for g in granted {
            if matches!(g, Capability::FsRead | Capability::FsWrite) {
                // Future: bind a sandboxed virtual filesystem rooted in the
                // project dir.
            }
        }
        let ctx = PluginCtx {
            wasi: wasi_builder.build(),
            table: ResourceTable::new(),
        };
        let store = Store::new(&self.engine, ctx);
        Ok((store, component))
    }

    /// Make a fresh linker with the WASI host functions registered.
    pub fn linker(&self) -> Result<Linker<PluginCtx>, PluginHostError> {
        let mut linker: Linker<PluginCtx> = Linker::new(&self.engine);
        wasmtime_wasi::p2::add_to_linker_async(&mut linker)
            .map_err(|e| PluginHostError::Engine(e.to_string()))?;
        Ok(linker)
    }
}

fn caps_match(granted: &Capability, requested: &Capability) -> bool {
    use Capability::*;
    match (granted, requested) {
        (Effects { kinds: g }, Effects { kinds: r }) => r.iter().all(|k| g.contains(k)),
        (ScoringFeatures { names: g }, ScoringFeatures { names: r }) => {
            r.iter().all(|n| g.contains(n))
        }
        (Exporters { formats: g }, Exporters { formats: r }) => r.iter().all(|f| g.contains(f)),
        (PromptPackStyles { styles: g }, PromptPackStyles { styles: r }) => {
            r.iter().all(|s| g.contains(s))
        }
        (FsRead, FsRead) => true,
        (FsWrite, FsWrite) => true,
        (
            Network {
                allow_hosts: granted_hosts,
            },
            Network {
                allow_hosts: requested_hosts,
            },
        ) => requested_hosts.iter().all(|h| granted_hosts.contains(h)),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn host_constructs() {
        let _ = PluginHost::new().unwrap();
    }

    #[test]
    fn capability_matching_is_strict() {
        let granted = Capability::Effects {
            kinds: vec!["a".into()],
        };
        let requested_ok = Capability::Effects {
            kinds: vec!["a".into()],
        };
        let requested_bad = Capability::Effects {
            kinds: vec!["a".into(), "b".into()],
        };
        assert!(caps_match(&granted, &requested_ok));
        assert!(!caps_match(&granted, &requested_bad));
    }
}
