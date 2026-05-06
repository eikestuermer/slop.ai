//! The strict JSON contract that the planner LLM must produce.
//!
//! Mirrors `packages/schemas/plan.v1.json`. The validator runs the JSON
//! Schema; this typed view is for ergonomic Rust usage after schema
//! validation passes.

use serde::{Deserialize, Serialize};

/// Top-level plan object.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Plan {
    /// Always `roughcut_plan.v1`.
    pub version: String,
    /// One-paragraph summary the model produced.
    pub summary: String,
    /// Proposed timeline.
    pub timeline: PlanTimeline,
    /// Optional captions to insert.
    #[serde(default)]
    pub captions: Vec<PlanCaption>,
    /// Warnings the model surfaced (e.g., "no good cutaway found").
    #[serde(default)]
    pub warnings: Vec<String>,
}

/// Proposed timeline shape.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PlanTimeline {
    /// Frame rate.
    pub fps: f64,
    /// Tracks.
    pub tracks: Vec<PlanTrack>,
}

/// Proposed track.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PlanTrack {
    /// `video` or `audio`.
    pub kind: String,
    /// Track id (chosen by the model from a provided list).
    pub id: String,
    /// Clips on this track.
    pub clips: Vec<PlannedClip>,
}

/// A single proposed clip.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PlannedClip {
    /// Asset id from the candidate set.
    pub asset_id: String,
    /// Optional transcript segment id.
    #[serde(default)]
    pub segment_id: Option<String>,
    /// Optional shot id.
    #[serde(default)]
    pub shot_id: Option<String>,
    /// Source-time in.
    pub src_in: f64,
    /// Source-time out.
    pub src_out: f64,
    /// Timeline-time in.
    pub timeline_in: f64,
    /// Lane on the track. 0 = primary.
    #[serde(default)]
    pub lane: i32,
    /// Why the model picked this clip.
    #[serde(default)]
    pub reason: String,
}

/// Proposed caption.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PlanCaption {
    /// Optional originating segment id.
    #[serde(default)]
    pub segment_id: Option<String>,
    /// Timeline-time in.
    pub timeline_in: f64,
    /// Timeline-time out.
    pub timeline_out: f64,
    /// Caption text.
    pub text: String,
}
