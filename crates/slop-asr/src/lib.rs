//! # slop-asr
//!
//! Automatic speech recognition with a pluggable backend.
//!
//! Slop AI is local-first, but we make a deliberate choice not to *force*
//! whisper.cpp on every environment. Two backends ship:
//!
//! - [`backend::placeholder::PlaceholderBackend`] - pure Rust, always
//!   available. Uses simple silence detection over the audio waveform to
//!   emit synthetic "segments" with empty `text`. Useful for development,
//!   tests, and as a fallback when whisper.cpp is not installed.
//! - `backend::whisper_cpp::WhisperCppBackend` - real ASR via whisper.cpp.
//!   Compiled in only with `--features whisper-cpp`; gated behind that
//!   feature flag so a clean checkout builds without CMake / a C++ toolchain.
//!
//! ## Chunking and VAD
//!
//! Whisper-family models internally process audio in 30-second windows. We
//! pre-chunk longer files in [`chunk::chunk_audio`] using simple silence-aware
//! boundaries so segments do not get cut mid-word. The placeholder backend
//! reuses the same chunker, so the rest of the pipeline does not care which
//! backend is in use.
//!
//! ## Outputs
//!
//! Both backends produce a [`Transcript`]: a list of [`Segment`]s with
//! `start_sec`, `end_sec`, optional speaker tag, and the recognized text.
//! Transcripts serialize to and from JSON so they can be cached on disk per
//! asset.

#![deny(missing_docs)]

pub mod backend;
pub mod chunk;
pub mod model;
pub mod transcript;

pub use backend::{AsrBackend, AsrJob, AsrOptions};
pub use model::ModelManager;
pub use transcript::{Segment, Transcript};
