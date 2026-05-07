//! # slop-multicam
//!
//! Sync multicam angles by audio cross-correlation. Given N video files of
//! the same event captured by different cameras with overlapping audio, find
//! per-angle time offsets that align them on a common multicam timeline.
//!
//! ## Algorithm
//!
//! 1. Decode each angle's audio to 16 kHz mono `f32` (via ffmpeg).
//! 2. Pick the longest angle as the reference.
//! 3. For each other angle, compute the cross-correlation against the
//!    reference using FFT-based correlation (`rustfft` + `realfft`) over the
//!    overlap region. This is O(N log N) per pair.
//! 4. The `argmax` of the correlation is the sync offset in samples;
//!    convert to seconds.
//!
//! Cross-correlation has been the textbook standard for syncing audio
//! tracks for decades; the FFT-based form (Wiener-Khinchin theorem) is what
//! Resolve and PluralEyes use under the hood. This crate is a SOTA Rust
//! implementation: pure `f32`, single allocation per pair, deterministic.

#![deny(missing_docs)]

pub mod sync;

pub use sync::{compute_sync_offsets, SyncResult};
