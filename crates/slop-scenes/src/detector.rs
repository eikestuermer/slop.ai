//! The `Detector` trait and the high-level [`detect_scenes`] entry point.

use crate::frames::{decode_decimated_rgb, FrameStream};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// A single detected scene / shot.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Scene {
    /// Stable id, e.g. `shot_0001`.
    pub scene_id: String,
    /// Start time in seconds.
    pub start_sec: f64,
    /// End time in seconds.
    pub end_sec: f64,
}

/// Common detector knobs.
#[derive(Debug, Default, Clone, Copy)]
pub struct DetectorOptions {}

/// A scene detector.
pub trait Detector {
    /// Run the detector on a pre-decoded frame stream.
    fn detect(&self, stream: &FrameStream, opts: &DetectorOptions) -> Vec<Scene>;
    /// Detector name (`"content"`, `"adaptive"`).
    fn name(&self) -> &'static str;
}

/// Top-level entry point: decode the source via ffmpeg and run the detector.
pub async fn detect_scenes<D: Detector>(
    input: impl AsRef<Path>,
    duration_sec: f64,
    detector: &D,
) -> Result<Vec<Scene>, crate::frames::FrameError> {
    let stream = decode_decimated_rgb(input, duration_sec, 5.0, 160, 90).await?;
    Ok(detector.detect(&stream, &DetectorOptions::default()))
}
