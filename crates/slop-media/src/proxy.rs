//! Generate low-resolution proxy renditions for snappy editing.
//!
//! Default proxy: 720p H.264 / AAC, fast preset, faststart, 23 CRF, audio 128k.

use crate::error::{MediaError, Result};
use std::path::{Path, PathBuf};
use tokio::process::Command;

/// Options controlling proxy generation.
#[derive(Debug, Clone)]
pub struct ProxyOptions {
    /// Target proxy height in pixels (width is derived to keep aspect ratio).
    pub height: u32,
    /// `libx264` `-crf` value. Lower = higher quality.
    pub crf: u32,
    /// libx264 `-preset` (`ultrafast`, `superfast`, ...).
    pub preset: &'static str,
    /// AAC audio bitrate, e.g. `"128k"`.
    pub audio_bitrate: &'static str,
}

impl Default for ProxyOptions {
    fn default() -> Self {
        Self {
            height: 720,
            crf: 23,
            preset: "veryfast",
            audio_bitrate: "128k",
        }
    }
}

/// Render a proxy MP4 to `out`. `out` is overwritten if it exists. Returns
/// the path that was written.
pub async fn generate_proxy(
    input: impl AsRef<Path>,
    out: impl AsRef<Path>,
    opts: &ProxyOptions,
) -> Result<PathBuf> {
    let input = input.as_ref();
    let out = out.as_ref().to_path_buf();
    if let Some(parent) = out.parent() {
        if !parent.as_os_str().is_empty() {
            tokio::fs::create_dir_all(parent).await?;
        }
    }
    let scale = format!("scale=-2:{}", opts.height);

    let status = Command::new("ffmpeg")
        .args(["-y", "-i"])
        .arg(input)
        .args(["-vf", &scale])
        .args([
            "-c:v",
            "libx264",
            "-preset",
            opts.preset,
            "-crf",
            &opts.crf.to_string(),
            "-pix_fmt",
            "yuv420p",
            "-movflags",
            "+faststart",
            "-c:a",
            "aac",
            "-b:a",
            opts.audio_bitrate,
        ])
        .arg(&out)
        .status()
        .await
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                MediaError::BinaryNotFound("ffmpeg")
            } else {
                MediaError::Io(e)
            }
        })?;

    if !status.success() {
        return Err(MediaError::NonZeroExit {
            binary: "ffmpeg",
            status: status.code().unwrap_or(-1),
            stderr: String::new(),
        });
    }

    Ok(out)
}
