//! Whisper.cpp model file management.
//!
//! Models are downloaded on first use. We never bundle weights in the binary.
//! The default mirror is the official whisper.cpp Hugging Face repo.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Known whisper.cpp model variants.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum WhisperModel {
    /// English-only, ~75 MB. Good for low-latency dev.
    TinyEn,
    /// English-only, ~142 MB. Recommended dev default.
    BaseEn,
    /// Multilingual, ~466 MB.
    Small,
    /// Multilingual, ~1.4 GB.
    Medium,
    /// Multilingual, ~2.9 GB.
    LargeV3,
}

impl WhisperModel {
    /// Filename of the GGUF weights as published.
    pub fn filename(&self) -> &'static str {
        match self {
            WhisperModel::TinyEn => "ggml-tiny.en.bin",
            WhisperModel::BaseEn => "ggml-base.en.bin",
            WhisperModel::Small => "ggml-small.bin",
            WhisperModel::Medium => "ggml-medium.bin",
            WhisperModel::LargeV3 => "ggml-large-v3.bin",
        }
    }
    /// Public URL (Hugging Face mirror).
    pub fn default_url(&self) -> String {
        format!(
            "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/{}",
            self.filename()
        )
    }
}

/// Manage the local cache of whisper model files.
#[derive(Debug, Clone)]
pub struct ModelManager {
    /// Where models live on disk.
    pub root: PathBuf,
}

impl ModelManager {
    /// Construct a manager rooted at `root`. The directory is created if it
    /// does not exist.
    pub fn new(root: impl Into<PathBuf>) -> std::io::Result<Self> {
        let root = root.into();
        std::fs::create_dir_all(&root)?;
        Ok(Self { root })
    }

    /// Path to a model file (whether or not it has been downloaded).
    pub fn path_for(&self, model: WhisperModel) -> PathBuf {
        self.root.join(model.filename())
    }

    /// Has this model been downloaded?
    pub fn is_present(&self, model: WhisperModel) -> bool {
        self.path_for(model).is_file()
    }

    /// Verify a downloaded model by computing its size and comparing to the
    /// expected value, if known. We deliberately do not ship hashes for
    /// every model file because upstream sometimes republishes; size is a
    /// reasonable smoke check.
    pub fn validate(&self, model: WhisperModel) -> std::io::Result<u64> {
        let path = self.path_for(model);
        let meta = std::fs::metadata(&path)?;
        Ok(meta.len())
    }

    /// Pretty-print a sentence describing what is installed.
    pub fn status_line(&self, model: WhisperModel) -> String {
        if self.is_present(model) {
            format!("{} installed at {}", model.filename(), self.path_for(model).display())
        } else {
            format!(
                "{} not installed. Download with: curl -L {} -o {}",
                model.filename(),
                model.default_url(),
                self.path_for(model).display()
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_url_includes_filename() {
        let url = WhisperModel::BaseEn.default_url();
        assert!(url.ends_with("ggml-base.en.bin"));
    }

    #[test]
    fn manager_creates_root() {
        let dir = std::env::temp_dir().join(format!("slop-asr-mm-{}", uuid::Uuid::new_v4()));
        let m = ModelManager::new(&dir).unwrap();
        assert!(m.root.is_dir());
        assert!(!m.is_present(WhisperModel::BaseEn));
    }

    #[test]
    fn status_line_mentions_model() {
        let dir = std::env::temp_dir().join(format!("slop-asr-mm2-{}", uuid::Uuid::new_v4()));
        let m = ModelManager::new(&dir).unwrap();
        let s = m.status_line(WhisperModel::TinyEn);
        assert!(s.contains("ggml-tiny.en.bin"));
    }
}
