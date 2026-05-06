//! whisper.cpp ASR backend.
//!
//! Compiled in only with `--features whisper-cpp`. This is a thin scaffold;
//! the actual library binding will be added when CI is set up to build
//! whisper.cpp via `whisper-rs`. The trait surface is identical to
//! [`crate::backend::placeholder::PlaceholderBackend`] so callers can swap
//! at runtime.

use crate::backend::{AsrBackend, AsrError, AsrJob, AsrOptions};
use crate::transcript::Transcript;
use async_trait::async_trait;
use std::path::PathBuf;

/// whisper.cpp backend (feature-gated).
#[derive(Debug, Clone)]
pub struct WhisperCppBackend {
    /// Path to a downloaded GGUF model file.
    pub model_path: PathBuf,
}

#[async_trait]
impl AsrBackend for WhisperCppBackend {
    fn name(&self) -> &'static str {
        "whisper-cpp"
    }
    async fn transcribe(
        &self,
        _job: AsrJob,
        _opts: &AsrOptions,
    ) -> Result<Transcript, AsrError> {
        // Wired-but-not-built. The full binding lives behind the
        // `whisper-cpp` feature and is added in a follow-up commit that
        // pulls the `whisper-rs` crate into the workspace.
        Err(AsrError::Backend(
            "whisper-cpp backend not yet implemented in this build".into(),
        ))
    }
}
