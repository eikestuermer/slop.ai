//! Generate a horizontal "thumb strip": N evenly spaced frames concatenated
//! into a single image for the timeline canvas.

use crate::error::{MediaError, Result};
use std::path::{Path, PathBuf};
use tokio::process::Command;

/// Options for thumb-strip generation.
#[derive(Debug, Clone)]
pub struct ThumbOptions {
    /// Number of frames in the strip.
    pub n_frames: u32,
    /// Height in pixels for each thumbnail.
    pub thumb_height: u32,
}

impl Default for ThumbOptions {
    fn default() -> Self {
        Self {
            n_frames: 60,
            thumb_height: 90,
        }
    }
}

/// Generate a horizontal thumb strip PNG from `input`.
///
/// `duration_sec` should come from a previous probe; we use it to compute the
/// `fps` filter to produce exactly `n_frames` frames.
pub async fn generate_thumb_strip(
    input: impl AsRef<Path>,
    duration_sec: f64,
    out: impl AsRef<Path>,
    opts: &ThumbOptions,
) -> Result<PathBuf> {
    let input = input.as_ref();
    let out = out.as_ref().to_path_buf();
    if let Some(parent) = out.parent() {
        if !parent.as_os_str().is_empty() {
            tokio::fs::create_dir_all(parent).await?;
        }
    }

    let fps = (opts.n_frames as f64 / duration_sec.max(0.1)).max(0.001);
    let filter = format!(
        "fps={fps},scale=-2:{h},tile={n}x1",
        fps = fps,
        h = opts.thumb_height,
        n = opts.n_frames,
    );

    let status = Command::new("ffmpeg")
        .args(["-y", "-i"])
        .arg(input)
        .args(["-vf", &filter, "-frames:v", "1"])
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
