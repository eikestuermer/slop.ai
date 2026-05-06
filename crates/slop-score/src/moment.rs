//! Candidate moment type.

use serde::{Deserialize, Serialize};

/// A single candidate moment.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Moment {
    /// Stable id, e.g. `m_<short>`.
    pub moment_id: String,
    /// Asset id this moment is from.
    pub asset_id: String,
    /// Start time in seconds.
    pub start_sec: f64,
    /// End time in seconds.
    pub end_sec: f64,
    /// Optional transcript segment id this moment was derived from.
    #[serde(default)]
    pub segment_id: Option<String>,
    /// Optional shot id this moment overlaps.
    #[serde(default)]
    pub shot_id: Option<String>,
    /// Optional speaker tag.
    #[serde(default)]
    pub speaker: Option<String>,
    /// Verbatim text from the underlying transcript segment, if any.
    #[serde(default)]
    pub text: String,
    /// Total weighted score in [0, 1].
    pub score: f32,
    /// Per-feature score breakdown (for debugging and UI).
    #[serde(default)]
    pub features: Vec<(String, f32)>,
}

/// Builder that collects raw signals and assembles `Moment`s.
#[derive(Debug, Default)]
pub struct MomentBuilder {
    moments: Vec<Moment>,
}

impl MomentBuilder {
    /// Empty builder.
    pub fn new() -> Self {
        Self::default()
    }
    /// Push a moment.
    pub fn push(&mut self, m: Moment) {
        self.moments.push(m);
    }
    /// All moments.
    pub fn into_vec(self) -> Vec<Moment> {
        self.moments
    }
}
