//! # slop-render
//!
//! Compile a [`slop_core::Timeline`] into a deterministic FFmpeg filtergraph
//! and run it to produce an MP4 preview.
//!
//! The render compiler is intentionally simple for V1:
//!
//! - one input file per unique `asset_id`,
//! - per clip: trim source range, scale to project resolution, set PTS,
//! - concat all video and audio segments per track,
//! - overlay all video tracks (lane 0 = primary, lane 1+ = cutaway/overlay),
//! - mix all audio tracks,
//! - drawtext overlays for every caption.
//!
//! Effect support is limited to `fade_in`, `fade_out`, and a simple
//! cross-dissolve approximated by overlapping adjacent clips with `fade`.
//! The doc on `docs/non-goals.md` is the source of truth for what this
//! compiler does not promise.

#![deny(missing_docs)]

pub mod compiler;
pub mod runner;

pub use compiler::{compile_timeline, CompiledRender, RenderOptions};
pub use runner::{render, RenderError};
