//! Project state owned by the Tauri host.
//!
//! Holds the in-memory `Timeline`, the asset cache (with proxy/transcript/scene
//! status), the path to the project directory, and the BYO LLM endpoint
//! configuration. Mutations always go through `slop_core::reducer` so the
//! op log on disk stays the single source of truth.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use slop_core::{ops::OpLog, Timeline};
use slop_planner::EndpointConfig;
use slop_score::Moment;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use tauri::AppHandle;

/// One asset's view from the frontend's perspective.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetView {
    /// Asset id.
    pub asset_id: String,
    /// Asset URI.
    pub uri: String,
    /// Duration.
    pub duration_sec: f64,
    /// Has video.
    pub has_video: bool,
    /// Has audio.
    pub has_audio: bool,
    /// Source video fps.
    pub fps: Option<f64>,
    /// Source resolution.
    pub resolution: Option<slop_core::Resolution>,
    /// Path to generated proxy MP4.
    pub proxy_path: Option<PathBuf>,
    /// Path to generated thumb-strip PNG.
    pub thumb_strip_path: Option<PathBuf>,
    /// Transcript job status.
    pub transcript_status: JobStatus,
    /// Scene-detection job status.
    pub scenes_status: JobStatus,
}

/// Job status enum used by the frontend.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum JobStatus {
    /// Not started.
    Missing,
    /// In flight.
    Running,
    /// Done.
    Ready,
    /// Failed.
    Error,
}

/// Full project view returned to the frontend.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectView {
    /// Project root on disk.
    pub path: PathBuf,
    /// Canonical timeline.
    pub timeline: Timeline,
    /// Per-asset metadata.
    pub assets: Vec<AssetView>,
    /// Number of background jobs in flight.
    pub pending_jobs: u32,
}

/// Per-asset background data the host caches.
#[derive(Debug, Clone)]
pub struct AssetCache {
    /// View for the frontend.
    pub view: AssetView,
    /// Most recent transcript, if any.
    pub transcript: Option<slop_asr::Transcript>,
    /// Most recent scene list.
    pub scenes: Vec<slop_scenes::Scene>,
}

/// Inner mutable host state.
#[derive(Debug)]
pub struct InnerState {
    /// Project root.
    pub path: PathBuf,
    /// Canonical timeline.
    pub timeline: Timeline,
    /// Op log mirroring `ops.jsonl`.
    pub op_log: OpLog,
    /// Per-asset cache.
    pub assets: BTreeMap<String, AssetCache>,
    /// Latest scored candidate moments (built by `build_candidates`).
    pub candidates: Vec<Moment>,
    /// BYO LLM endpoint config.
    pub endpoint: EndpointConfig,
    /// Privacy mode: when true, only `http://localhost`, `127.0.0.1`, and
    /// `[::1]` URLs are accepted as endpoints, and any networked feature
    /// outside that allow-list is gated.
    pub privacy_mode: bool,
    /// Number of in-flight jobs (for the spinner).
    pub pending_jobs: u32,
}

/// Shared application state.
#[derive(Debug, Clone)]
pub struct AppState {
    inner: Arc<Mutex<InnerState>>,
    /// Tauri handle (used for emitting events; reserved for V1.1).
    pub _app: AppHandle,
}

impl AppState {
    /// Create an empty AppState rooted at the user's data dir.
    pub fn new(app: AppHandle) -> Result<Self> {
        let path = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let inner = InnerState {
            path,
            timeline: Timeline::empty(),
            op_log: OpLog::new(),
            assets: BTreeMap::new(),
            candidates: Vec::new(),
            endpoint: EndpointConfig::default(),
            privacy_mode: false,
            pending_jobs: 0,
        };
        Ok(Self {
            inner: Arc::new(Mutex::new(inner)),
            _app: app,
        })
    }

    /// Apply an op and persist it to `ops.jsonl`.
    pub fn apply_and_log(&self, op: slop_core::Op) -> Result<()> {
        let mut inner = self.inner.lock().unwrap();
        slop_core::reducer::apply(&mut inner.timeline, &op)
            .map_err(|e| anyhow::anyhow!(e.to_string()))?;
        let ops_path = inner.path.join("ops.jsonl");
        OpLog::append_to_file(&op, &ops_path)
            .map_err(|e| anyhow::anyhow!(e.to_string()))?;
        inner.op_log.push(op);
        Ok(())
    }

    /// Load a project from disk.
    pub fn load_from(&self, root: &Path) -> Result<()> {
        let mut inner = self.inner.lock().unwrap();
        inner.path = root.to_path_buf();
        std::fs::create_dir_all(root).context("create project dir")?;

        let ops_path = root.join("ops.jsonl");
        if ops_path.is_file() {
            let log = OpLog::load(&ops_path).map_err(|e| anyhow::anyhow!(e.to_string()))?;
            let tl = slop_core::reducer::replay(log.ops())
                .map_err(|e| anyhow::anyhow!(e.to_string()))?;
            inner.timeline = tl;
            inner.op_log = log;
        } else {
            inner.timeline = Timeline::empty();
            inner.op_log = OpLog::new();
        }
        Ok(())
    }

    /// Construct a frontend-facing `ProjectView`.
    pub fn snapshot(&self) -> ProjectView {
        let inner = self.inner.lock().unwrap();
        ProjectView {
            path: inner.path.clone(),
            timeline: inner.timeline.clone(),
            assets: inner.assets.values().map(|a| a.view.clone()).collect(),
            pending_jobs: inner.pending_jobs,
        }
    }

    /// Locked access to the inner state. Panics on poison; this is acceptable
    /// for a desktop app where panics terminate the process.
    pub fn with<R>(&self, f: impl FnOnce(&mut InnerState) -> R) -> R {
        let mut inner = self.inner.lock().unwrap();
        f(&mut inner)
    }
}
