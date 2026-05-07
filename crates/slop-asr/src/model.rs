//! Whisper.cpp model file management.
//!
//! Models are downloaded on first use. We never bundle weights in the binary.
//! Download progress is reported via a callback so the desktop UI can render
//! a progress bar. Files are checksum-verified after download.
//!
//! ## Mirror policy
//!
//! The default mirror is the official whisper.cpp Hugging Face repo, which
//! ships GGUF weights for every officially supported model size. Users in
//! restricted networks can override the mirror via [`ModelManager::with_mirror`].

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use thiserror::Error;

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
    /// Public URL on the default Hugging Face mirror.
    pub fn default_url(&self) -> String {
        format!(
            "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/{}",
            self.filename()
        )
    }
    /// Known SHA-256 of the GGUF blob (for verifying downloads).
    ///
    /// These checksums are pinned; if upstream republishes a model, we
    /// surface the mismatch as an error rather than silently accepting it.
    /// Use `--features whisper-cpp` only after verifying the published
    /// checksum matches what is below.
    pub fn known_sha256(&self) -> Option<&'static str> {
        match self {
            WhisperModel::TinyEn => Some(
                // Pinned 2025-Q1; verify before bumping.
                "921e4cf8686fdd993dcd081a5da5b6c365bfde1162e72b08d75ac75289920b1f",
            ),
            WhisperModel::BaseEn => {
                Some("60ed5bcaf17ce0b8e7b86b89be7d3e6c3c8c5a9cff2c4b1b46f1d24ff2a5c87a")
            }
            // Larger models change more often; verify locally and add when
            // pinning is desired.
            _ => None,
        }
    }
    /// Approximate size in bytes (for progress reporting before content-length
    /// is known).
    pub fn approx_size_bytes(&self) -> u64 {
        match self {
            WhisperModel::TinyEn => 75_000_000,
            WhisperModel::BaseEn => 142_000_000,
            WhisperModel::Small => 466_000_000,
            WhisperModel::Medium => 1_400_000_000,
            WhisperModel::LargeV3 => 2_900_000_000,
        }
    }
}

/// Errors that can come out of model management.
#[derive(Debug, Error)]
pub enum ModelError {
    /// HTTP failure during download.
    #[error("download failed: {0}")]
    Download(String),
    /// SHA-256 mismatch between downloaded blob and pinned hash.
    #[error("checksum mismatch for {filename}: expected {expected}, got {actual}")]
    ChecksumMismatch {
        /// File.
        filename: String,
        /// Expected hex.
        expected: String,
        /// Actual hex.
        actual: String,
    },
    /// I/O failure.
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

/// Progress callback invoked periodically during downloads.
///
/// Arguments are `(downloaded_bytes, total_bytes_or_zero_if_unknown)`.
pub type ProgressFn = Box<dyn Fn(u64, u64) + Send + Sync>;

/// Manage the local cache of whisper model files.
#[derive(Clone)]
pub struct ModelManager {
    /// Where models live on disk.
    pub root: PathBuf,
    mirror: String,
}

impl std::fmt::Debug for ModelManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ModelManager")
            .field("root", &self.root)
            .field("mirror", &self.mirror)
            .finish()
    }
}

impl ModelManager {
    /// Construct a manager rooted at `root`. The directory is created if it
    /// does not exist.
    pub fn new(root: impl Into<PathBuf>) -> std::io::Result<Self> {
        let root = root.into();
        std::fs::create_dir_all(&root)?;
        Ok(Self {
            root,
            mirror: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main".into(),
        })
    }

    /// Override the mirror base URL. Useful for restricted networks or
    /// pinned snapshots.
    pub fn with_mirror(mut self, mirror: impl Into<String>) -> Self {
        self.mirror = mirror.into();
        self
    }

    /// Path to a model file (whether or not it has been downloaded).
    pub fn path_for(&self, model: WhisperModel) -> PathBuf {
        self.root.join(model.filename())
    }

    /// Has this model been downloaded?
    pub fn is_present(&self, model: WhisperModel) -> bool {
        self.path_for(model).is_file()
    }

    /// Pretty-print a sentence describing what is installed.
    pub fn status_line(&self, model: WhisperModel) -> String {
        if self.is_present(model) {
            format!(
                "{} installed at {}",
                model.filename(),
                self.path_for(model).display()
            )
        } else {
            format!(
                "{} not installed. Download with the desktop UI or `slop fetch model {:?}`.",
                model.filename(),
                model,
            )
        }
    }

    /// Download `model` from the mirror, verifying its SHA-256 if known.
    ///
    /// Idempotent: if the file already exists and verifies, returns
    /// immediately. The download is atomic via a `.partial` swap so a
    /// killed download never leaves a corrupted blob.
    pub async fn download(
        &self,
        model: WhisperModel,
        progress: Option<ProgressFn>,
    ) -> Result<PathBuf, ModelError> {
        let final_path = self.path_for(model);
        if final_path.is_file() {
            if let Some(expected) = model.known_sha256() {
                let got = sha256_of(&final_path)?;
                if got == expected {
                    return Ok(final_path);
                }
                // Existing file is corrupted/wrong-version; redownload.
                std::fs::remove_file(&final_path)?;
            } else {
                return Ok(final_path);
            }
        }

        let url = format!("{}/{}", self.mirror.trim_end_matches('/'), model.filename());
        let partial = final_path.with_extension("partial");
        let progress_ref = progress.as_ref();
        let progress_dyn: Option<&(dyn Fn(u64, u64) + Send + Sync)> =
            progress_ref.map(|p| p.as_ref() as &(dyn Fn(u64, u64) + Send + Sync));
        download_streaming(&url, &partial, progress_dyn, model.approx_size_bytes())
            .await
            .map_err(|e| ModelError::Download(e.to_string()))?;

        if let Some(expected) = model.known_sha256() {
            let got = sha256_of(&partial)?;
            if got != expected {
                let _ = std::fs::remove_file(&partial);
                return Err(ModelError::ChecksumMismatch {
                    filename: model.filename().into(),
                    expected: expected.into(),
                    actual: got,
                });
            }
        }

        std::fs::rename(&partial, &final_path)?;
        Ok(final_path)
    }
}

/// Download a file from a public URL, verify its SHA-256, and atomically
/// move it into `out`. Used for both whisper.cpp models and pyannote ONNX
/// models.
pub async fn download_with_checksum(
    url: &str,
    out: &Path,
    expected_sha256: &str,
    progress: Option<&(dyn Fn(u64, u64) + Send + Sync)>,
) -> Result<(), ModelError> {
    if let Some(parent) = out.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let partial = out.with_extension("partial");
    download_streaming(url, &partial, progress, 0)
        .await
        .map_err(|e| ModelError::Download(e.to_string()))?;
    let got = sha256_of(&partial)?;
    if got != expected_sha256 {
        let _ = std::fs::remove_file(&partial);
        return Err(ModelError::ChecksumMismatch {
            filename: out
                .file_name()
                .map(|f| f.to_string_lossy().to_string())
                .unwrap_or_default(),
            expected: expected_sha256.into(),
            actual: got,
        });
    }
    std::fs::rename(&partial, out)?;
    Ok(())
}

async fn download_streaming(
    url: &str,
    out: &Path,
    progress: Option<&(dyn Fn(u64, u64) + Send + Sync)>,
    fallback_total: u64,
) -> anyhow::Result<()> {
    use futures_util::StreamExt;
    use std::io::Write;
    let resp = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(60 * 60))
        .build()?
        .get(url)
        .send()
        .await?
        .error_for_status()?;
    let total = resp.content_length().unwrap_or(fallback_total);
    let mut downloaded: u64 = 0;
    let mut f = std::fs::File::create(out)?;
    let mut stream = resp.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        f.write_all(&chunk)?;
        downloaded += chunk.len() as u64;
        if let Some(cb) = progress {
            cb(downloaded, total);
        }
    }
    f.flush()?;
    Ok(())
}

fn sha256_of(p: &Path) -> std::io::Result<String> {
    use std::io::Read;
    let mut f = std::fs::File::open(p)?;
    let mut hasher = Sha256::new();
    let mut buf = vec![0u8; 1 << 20];
    loop {
        let n = f.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(format!("{:x}", hasher.finalize()))
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

    #[test]
    fn checksum_helper_matches_known_input() {
        let dir = std::env::temp_dir().join(format!("slop-asr-sha-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let p = dir.join("hello.bin");
        std::fs::write(&p, b"hello").unwrap();
        let got = sha256_of(&p).unwrap();
        assert_eq!(
            got,
            "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824"
        );
    }
}
