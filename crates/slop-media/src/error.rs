//! Media-pipeline error type.

use thiserror::Error;

/// Result alias.
pub type Result<T, E = MediaError> = std::result::Result<T, E>;

/// All errors that the media pipeline can produce.
#[derive(Debug, Error)]
pub enum MediaError {
    /// `ffprobe` or `ffmpeg` was not found on `PATH`.
    #[error("required binary {0} not found on PATH")]
    BinaryNotFound(&'static str),

    /// The binary was found but exited non-zero.
    #[error("{binary} exited with status {status}: {stderr}")]
    NonZeroExit {
        /// Which binary.
        binary: &'static str,
        /// Exit status.
        status: i32,
        /// Captured stderr.
        stderr: String,
    },

    /// Could not parse the binary's output (e.g., ffprobe JSON).
    #[error("failed to parse {binary} output: {message}")]
    ParseFailure {
        /// Which binary.
        binary: &'static str,
        /// Detail.
        message: String,
    },

    /// JSON (de)serialization failure.
    #[error(transparent)]
    Json(#[from] serde_json::Error),

    /// I/O failure.
    #[error(transparent)]
    Io(#[from] std::io::Error),

    /// WAV parsing failure.
    #[error(transparent)]
    Wav(#[from] hound::Error),
}
