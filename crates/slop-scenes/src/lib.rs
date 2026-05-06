//! # slop-scenes
//!
//! Shot / scene boundary detection.
//!
//! This crate is a faithful Rust port of the algorithm used by
//! PySceneDetect's `ContentDetector`: compute a per-frame "content score" as
//! the mean absolute pixel-difference in HSV between adjacent decimated
//! frames; emit a cut whenever the score exceeds a threshold and the
//! previous cut is at least `min_scene_len` frames behind us.
//!
//! `AdaptiveDetector` extends `ContentDetector` by computing the threshold
//! as `rolling_mean * adaptive_ratio` over a sliding window. This survives
//! the camera-pan vs. true-cut ambiguity better than a fixed threshold.
//!
//! ## How frames are decoded
//!
//! For V1 we shell out to `ffmpeg` to decimate and rescale the input to a
//! small (`160x90` by default) RGB raw stream and pipe it on stdout. This
//! avoids bringing libavcodec or an image crate into the build, keeps the
//! algorithm independent of the decoder, and stays close to PySceneDetect's
//! reference implementation.
//!
//! See [`detect_scenes`] for the public entry point.

#![deny(missing_docs)]

pub mod adaptive;
pub mod content;
pub mod detector;
pub mod frames;

pub use adaptive::AdaptiveDetector;
pub use content::ContentDetector;
pub use detector::{detect_scenes, Detector, DetectorOptions, Scene};
pub use frames::{decode_decimated_rgb, FrameStream, RgbFrame};
