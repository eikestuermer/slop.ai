//! Transcript types.

use serde::{Deserialize, Serialize};
use std::path::Path;

/// A single transcript segment.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Segment {
    /// Stable id, e.g. `seg_<short>`. Used by the candidate builder and
    /// by the planner LLM as a candidate handle.
    pub segment_id: String,
    /// Start time in seconds from the beginning of the asset.
    pub start_sec: f64,
    /// End time in seconds.
    pub end_sec: f64,
    /// Optional speaker tag from diarization (`"S1"`, `"S2"`, ...).
    #[serde(default)]
    pub speaker: Option<String>,
    /// Recognized text. May be empty for placeholder/silence segments.
    pub text: String,
    /// ASR confidence in [0, 1] when the backend exposes it.
    #[serde(default)]
    pub confidence: Option<f32>,
}

/// Full transcript for a single asset.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Transcript {
    /// Asset id this transcript belongs to.
    pub asset_id: String,
    /// Backend that produced it (`placeholder`, `whisper-cpp`).
    pub backend: String,
    /// Model name where applicable (`whisper-base.en`, `placeholder`).
    pub model: String,
    /// Detected language code (`en`, `de`, ...).
    #[serde(default)]
    pub language: Option<String>,
    /// All segments, ordered by `start_sec`.
    pub segments: Vec<Segment>,
}

impl Transcript {
    /// Save to JSON.
    pub fn save_json(&self, path: impl AsRef<Path>) -> std::io::Result<()> {
        let s = serde_json::to_string_pretty(self).expect("transcript serializes");
        std::fs::write(path, s)
    }
    /// Load from JSON.
    pub fn load_json(path: impl AsRef<Path>) -> std::io::Result<Self> {
        let s = std::fs::read_to_string(path)?;
        let t: Self = serde_json::from_str(&s)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        Ok(t)
    }
    /// Return total spoken duration in seconds (sum of segment durations).
    pub fn duration_sec(&self) -> f64 {
        self.segments
            .iter()
            .map(|s| (s.end_sec - s.start_sec).max(0.0))
            .sum()
    }
}
