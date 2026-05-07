//! ASR backends.
//!
//! See [`AsrBackend`] for the trait every backend must implement.

pub mod placeholder;

#[cfg(feature = "whisper-cpp")]
pub mod whisper_cpp;

use crate::transcript::Transcript;
use std::path::PathBuf;

/// A single transcription job.
#[derive(Debug, Clone)]
pub struct AsrJob {
    /// Asset id this transcript will belong to.
    pub asset_id: String,
    /// Path to the source media file.
    pub input: PathBuf,
    /// Total duration in seconds (from probe).
    pub duration_sec: f64,
}

/// Tunables shared across backends.
#[derive(Debug, Clone)]
pub struct AsrOptions {
    /// Model name where applicable.
    pub model: String,
    /// Language code (`"en"`, `"de"`, `"auto"`).
    pub language: String,
    /// Whether to apply silence-aware chunking before recognition.
    pub silence_chunk: bool,
}

impl Default for AsrOptions {
    fn default() -> Self {
        Self {
            model: "placeholder".into(),
            language: "auto".into(),
            silence_chunk: true,
        }
    }
}

/// A speech-to-text backend.
#[async_trait::async_trait]
pub trait AsrBackend: Send + Sync {
    /// Backend name (`"placeholder"`, `"whisper-cpp"`).
    fn name(&self) -> &'static str;

    /// Run a single transcription job.
    async fn transcribe(&self, job: AsrJob, opts: &AsrOptions) -> Result<Transcript, AsrError>;
}

/// Errors any backend can produce.
#[derive(Debug, thiserror::Error)]
pub enum AsrError {
    /// Underlying media error (could not extract audio, etc.).
    #[error(transparent)]
    Media(#[from] slop_media::MediaError),
    /// Backend-specific failure.
    #[error("{0}")]
    Backend(String),
    /// I/O.
    #[error(transparent)]
    Io(#[from] std::io::Error),
    /// JSON.
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}
