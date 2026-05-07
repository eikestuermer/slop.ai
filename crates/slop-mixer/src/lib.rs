//! # slop-mixer
//!
//! Audio mixer and loudness metering for Slop AI.
//!
//! ## LUFS metering
//!
//! Implements ITU-R BS.1770-4 ("Algorithms to measure audio programme
//! loudness and true-peak audio level"), the recognized standard. Three
//! metrics are produced:
//!
//! - **Integrated loudness (LUFS)**: long-term loudness over the whole
//!   programme, used as the primary delivery target.
//! - **Loudness Range (LRA)**: LU-units between the 10th and 95th
//!   percentile of short-term loudness; describes dynamic range.
//! - **True Peak (dBTP)**: oversampled inter-sample peak.
//!
//! Targets for delivery (encoded in the v2 schema's `Mixer.loudness_target`):
//!
//! - YouTube / Spotify / Apple Music: **-14 LUFS, -1 dBTP**.
//! - EBU R128 broadcast: **-23 LUFS, -1 dBTP**.
//! - AES streaming: **-16 LUFS** (legacy reference).
//!
//! ## Filtergraph emission
//!
//! For final render we emit FFmpeg's `loudnorm` filter with two-pass
//! parameters baked in so the output hits the configured target precisely
//! (the BS.1770-4 algorithm requires a measurement pass first; we drive
//! ffmpeg via `loudnorm=measured_*=...:print_format=json` and then bake
//! the measured values back into the actual render).

#![deny(missing_docs)]

pub mod ffmpeg;
pub mod loudness;

pub use ffmpeg::loudnorm_filtergraph;
pub use loudness::{measure_loudness, LoudnessMetrics, LoudnessTarget};
