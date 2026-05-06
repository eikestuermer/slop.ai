//! # slop-media
//!
//! Thin async wrappers over the system `ffmpeg` and `ffprobe` binaries.
//! Slop AI deliberately shells out instead of linking the FFmpeg C API:
//!
//! - dramatically simpler licensing posture (LGPL dynamic-link),
//! - smaller binary,
//! - easier to upgrade FFmpeg without rebuilding,
//! - reproducible behavior across desktop platforms.
//!
//! All public functions are async and rely on `tokio::process::Command`.
//! Errors are surfaced as [`MediaError`].

#![deny(missing_docs)]

pub mod error;
pub mod probe;
pub mod proxy;
pub mod thumbs;
pub mod waveform;

pub use error::{MediaError, Result};
pub use probe::{probe_asset, ProbeResult};
pub use proxy::{generate_proxy, ProxyOptions};
pub use thumbs::{generate_thumb_strip, ThumbOptions};
pub use waveform::{generate_waveform_peaks, WaveformOptions, WaveformPeaks};
