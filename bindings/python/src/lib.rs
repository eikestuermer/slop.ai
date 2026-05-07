//! Python bindings for Slop AI.
//!
//! Build with:
//!
//! ```text
//! pip install maturin
//! maturin develop --release --manifest-path bindings/python/Cargo.toml
//! ```

#![allow(clippy::useless_conversion)]

use pyo3::prelude::*;
use slop_core::Timeline;

/// A Slop AI timeline. Wraps `slop_core::Timeline` and exposes the
/// fundamental operations Python users need: load from disk, save, render.
#[pyclass]
#[derive(Clone)]
pub struct PyTimeline {
    inner: Timeline,
}

#[pymethods]
impl PyTimeline {
    #[new]
    fn new() -> Self {
        Self {
            inner: Timeline::empty(),
        }
    }

    /// Load from an `ops.jsonl`.
    #[staticmethod]
    fn load_ops(path: &str) -> PyResult<Self> {
        let log = slop_core::OpLog::load(path)
            .map_err(|e| pyo3::exceptions::PyIOError::new_err(e.to_string()))?;
        let inner = slop_core::reducer::replay(log.ops())
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
        Ok(Self { inner })
    }

    /// Total duration in seconds.
    fn duration_sec(&self) -> f64 {
        self.inner.duration_sec()
    }

    /// Number of tracks.
    fn n_tracks(&self) -> usize {
        self.inner.tracks.len()
    }

    /// Export to OTIO at `path`.
    fn export_otio(&self, path: &str) -> PyResult<()> {
        slop_otio::write_otio(&self.inner, std::path::Path::new(path))
            .map_err(|e| pyo3::exceptions::PyIOError::new_err(e.to_string()))?;
        Ok(())
    }

    /// JSON repr.
    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string_pretty(&self.inner)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }
}

#[pymodule]
fn slop_py(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyTimeline>()?;
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    Ok(())
}
