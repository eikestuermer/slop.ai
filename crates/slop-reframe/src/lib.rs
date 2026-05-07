//! # slop-reframe
//!
//! Smart reframing for vertical (9:16) and square (1:1) aspect-ratio
//! delivery. Used to turn 16:9 footage into TikTok / Reels / Shorts
//! versions without manual keyframing.
//!
//! ## Pipeline
//!
//! 1. Decimated frame extraction (5 fps via ffmpeg, same as slop-scenes).
//! 2. Per-frame subject detection: YOLOv11-nano via ONNX Runtime
//!    (`yolo11n.onnx`, ~6 MB, Apache-2.0). The "subject" is the
//!    highest-confidence detection in the `person` class, falling back to
//!    saliency centroid if no person is found.
//! 3. Temporal smoothing: 1D Kalman filter on the subject's centroid
//!    coordinates so the resulting crop pans smoothly rather than
//!    jittering frame-to-frame.
//! 4. Crop-window solver: given the detection track and the target aspect,
//!    compute a per-frame `(x, y, w, h)` that keeps the subject in frame.
//! 5. Filtergraph emission: an ffmpeg `crop=` chain with `eval=frame` and
//!    a sidecar `geq` so the crop center follows the smoothed track.

#![deny(missing_docs)]

pub mod kalman;
pub mod solver;
pub mod yolo;

pub use kalman::Kalman1D;
pub use solver::{compute_crop_track, CropFrame, ReframeOptions};
pub use yolo::{Detection, YoloDetector};
