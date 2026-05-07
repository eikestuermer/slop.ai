//! Node.js bindings via napi-rs.

#![deny(clippy::all)]

use napi_derive::napi;
use slop_core::Timeline;

/// JS-facing timeline handle.
#[napi]
pub struct JsTimeline {
    inner: Timeline,
}

impl Default for JsTimeline {
    fn default() -> Self {
        Self::new()
    }
}

#[napi]
impl JsTimeline {
    #[napi(constructor)]
    pub fn new() -> Self {
        Self {
            inner: Timeline::empty(),
        }
    }

    #[napi(factory)]
    pub fn load_ops(path: String) -> napi::Result<Self> {
        let log =
            slop_core::OpLog::load(&path).map_err(|e| napi::Error::from_reason(e.to_string()))?;
        let inner = slop_core::reducer::replay(log.ops())
            .map_err(|e| napi::Error::from_reason(e.to_string()))?;
        Ok(Self { inner })
    }

    #[napi]
    pub fn duration_sec(&self) -> f64 {
        self.inner.duration_sec()
    }

    #[napi]
    pub fn n_tracks(&self) -> u32 {
        self.inner.tracks.len() as u32
    }

    #[napi]
    pub fn export_otio(&self, path: String) -> napi::Result<()> {
        slop_otio::write_otio(&self.inner, std::path::Path::new(&path))
            .map_err(|e| napi::Error::from_reason(e.to_string()))?;
        Ok(())
    }

    #[napi]
    pub fn to_json(&self) -> napi::Result<String> {
        serde_json::to_string_pretty(&self.inner)
            .map_err(|e| napi::Error::from_reason(e.to_string()))
    }
}
