//! Minimal OTIO JSON schema types.
//!
//! Only the fields and OTIO_SCHEMA tags Slop AI emits for V1 are modeled.
//! Unknown fields read by other applications are preserved by emitting
//! the official tags.

use serde::Serialize;

/// Rational time value used by OTIO to avoid floating-point drift.
#[derive(Debug, Clone, Serialize)]
pub struct RationalTime {
    /// OTIO type tag.
    #[serde(rename = "OTIO_SCHEMA")]
    pub otio_schema: &'static str,
    /// Frame count.
    pub value: f64,
    /// Frames per second.
    pub rate: f64,
}

impl RationalTime {
    /// Build a RationalTime from a duration in seconds at `fps`.
    pub fn from_secs(secs: f64, fps: f64) -> Self {
        Self {
            otio_schema: "RationalTime.1",
            value: secs * fps,
            rate: fps,
        }
    }
}

/// Half-open `[start, start + duration)` time interval.
#[derive(Debug, Clone, Serialize)]
pub struct TimeRange {
    /// OTIO type tag.
    #[serde(rename = "OTIO_SCHEMA")]
    pub otio_schema: &'static str,
    /// Start of the range.
    pub start_time: RationalTime,
    /// Duration.
    pub duration: RationalTime,
}

impl TimeRange {
    /// Build a TimeRange from `[start, end)` seconds at `fps`.
    pub fn from_secs(start: f64, end: f64, fps: f64) -> Self {
        Self {
            otio_schema: "TimeRange.1",
            start_time: RationalTime::from_secs(start, fps),
            duration: RationalTime::from_secs((end - start).max(0.0), fps),
        }
    }
}

/// External reference: pointer to a media file on disk.
#[derive(Debug, Clone, Serialize)]
pub struct ExternalReference {
    /// OTIO type tag.
    #[serde(rename = "OTIO_SCHEMA")]
    pub otio_schema: &'static str,
    /// Target URL (`file:///...`).
    pub target_url: String,
    /// Available range of the underlying media.
    pub available_range: TimeRange,
    /// Free-form metadata.
    pub metadata: serde_json::Value,
}

/// Marker.
#[derive(Debug, Clone, Serialize)]
pub struct Marker {
    /// OTIO type tag.
    #[serde(rename = "OTIO_SCHEMA")]
    pub otio_schema: &'static str,
    /// Human label.
    pub name: String,
    /// Color name.
    pub color: String,
    /// Range the marker covers (instantaneous markers use duration=0).
    pub marked_range: TimeRange,
    /// Free-form metadata.
    pub metadata: serde_json::Value,
}

/// Clip referencing an `ExternalReference`.
#[derive(Debug, Clone, Serialize)]
pub struct Clip {
    /// OTIO type tag.
    #[serde(rename = "OTIO_SCHEMA")]
    pub otio_schema: &'static str,
    /// Display name.
    pub name: String,
    /// Sub-range of the source media to use.
    pub source_range: TimeRange,
    /// Pointer to source media.
    pub media_reference: ExternalReference,
    /// Markers attached to this clip.
    pub markers: Vec<Marker>,
    /// Effects (V1: empty).
    pub effects: Vec<serde_json::Value>,
    /// Metadata.
    pub metadata: serde_json::Value,
}

/// Empty range on a track.
#[derive(Debug, Clone, Serialize)]
pub struct Gap {
    /// OTIO type tag.
    #[serde(rename = "OTIO_SCHEMA")]
    pub otio_schema: &'static str,
    /// Display name.
    pub name: String,
    /// How long the gap lasts.
    pub source_range: TimeRange,
}

/// Track item: clip or gap.
#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum TrackChild {
    /// A clip.
    Clip(Clip),
    /// A gap.
    Gap(Gap),
}

/// Track.
#[derive(Debug, Clone, Serialize)]
pub struct Track {
    /// OTIO type tag.
    #[serde(rename = "OTIO_SCHEMA")]
    pub otio_schema: &'static str,
    /// Display name.
    pub name: String,
    /// `Video` or `Audio`.
    pub kind: String,
    /// Child items.
    pub children: Vec<TrackChild>,
    /// Free-form metadata.
    pub metadata: serde_json::Value,
}

/// Stack of tracks. Slop AI uses a single stack containing all tracks.
#[derive(Debug, Clone, Serialize)]
pub struct Stack {
    /// OTIO type tag.
    #[serde(rename = "OTIO_SCHEMA")]
    pub otio_schema: &'static str,
    /// Display name.
    pub name: String,
    /// Tracks in this stack.
    pub children: Vec<Track>,
    /// Free-form metadata.
    pub metadata: serde_json::Value,
}

/// Top-level OTIO timeline.
#[derive(Debug, Clone, Serialize)]
pub struct Timeline {
    /// OTIO type tag.
    #[serde(rename = "OTIO_SCHEMA")]
    pub otio_schema: &'static str,
    /// Display name.
    pub name: String,
    /// Global start time on the master timeline.
    pub global_start_time: RationalTime,
    /// Stack containing all tracks.
    pub tracks: Stack,
    /// Free-form metadata.
    pub metadata: serde_json::Value,
}
