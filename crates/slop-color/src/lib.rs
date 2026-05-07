//! # slop-color
//!
//! Color pipeline for Slop AI: 3D LUT loading (Iridas `.cube`), CDL-form
//! primary/secondary correction (lift / gamma / gain / saturation /
//! contrast / white balance), and FFmpeg filtergraph emission for the
//! render compiler.
//!
//! The math follows the ASC CDL specification (American Society of
//! Cinematographers Color Decision List). This is the same primary
//! correction model that DaVinci Resolve, Premiere Lumetri, FCP, and the
//! ACES pipeline all share, so grades round-trip cleanly.
//!
//! Scopes (waveform, vectorscope, parade) are generated via FFmpeg's
//! `signalstats`, `waveform`, and `vectorscope` filters. We don't reinvent
//! the math; we drive ffmpeg with the right filtergraph.

#![deny(missing_docs)]

pub mod cdl;
pub mod ffmpeg;
pub mod lut;
pub mod scopes;

pub use cdl::{apply_cdl_pixel, ColorDecisionList};
pub use ffmpeg::{cdl_to_filtergraph, lut_to_filtergraph};
pub use lut::{load_cube_file, LutError, ThreeDLut};
pub use scopes::{ScopeKind, ScopeOptions};
