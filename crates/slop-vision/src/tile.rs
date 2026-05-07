//! Frame-tile extractor.

use base64::Engine;
use serde::{Deserialize, Serialize};
use std::path::Path;
use thiserror::Error;
use tokio::process::Command;

/// One extracted frame.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrameTile {
    /// Source asset id.
    pub asset_id: String,
    /// Source-time timestamp in seconds.
    pub t_sec: f64,
    /// JPEG bytes.
    #[serde(skip_serializing, skip_deserializing)]
    pub jpeg: Vec<u8>,
}

impl FrameTile {
    /// Render as a chat-completions image part.
    /// Compatible with OpenAI, Ollama (vision builds), llama.cpp server,
    /// and Qwen2.5-VL / Llava-OneVision endpoints.
    pub fn as_image_part(&self) -> serde_json::Value {
        let b64 = base64::engine::general_purpose::STANDARD.encode(&self.jpeg);
        serde_json::json!({
            "type": "image_url",
            "image_url": {
                "url": format!("data:image/jpeg;base64,{b64}")
            }
        })
    }
}

/// Extraction options.
#[derive(Debug, Clone)]
pub struct TileOptions {
    /// Tile size in pixels (square).
    pub tile_px: u32,
    /// JPEG quality (1..=100).
    pub jpeg_quality: u32,
}

impl Default for TileOptions {
    fn default() -> Self {
        Self {
            tile_px: 448,
            jpeg_quality: 80,
        }
    }
}

/// Errors during frame extraction.
#[derive(Debug, Error)]
pub enum TileError {
    /// ffmpeg not on PATH.
    #[error("ffmpeg not found on PATH")]
    BinaryNotFound,
    /// ffmpeg exited non-zero.
    #[error("ffmpeg exited {0}")]
    NonZeroExit(i32),
    /// I/O.
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

/// Extract one frame per timestamp from `input` at the given source-times
/// and return the JPEG-encoded tiles. We do this in a single ffmpeg
/// invocation per asset for I/O efficiency.
pub async fn frames_at_timestamps(
    input: impl AsRef<Path>,
    asset_id: &str,
    timestamps_sec: &[f64],
    opts: &TileOptions,
) -> Result<Vec<FrameTile>, TileError> {
    let input = input.as_ref();
    if timestamps_sec.is_empty() {
        return Ok(Vec::new());
    }
    let dir = std::env::temp_dir().join(format!(
        "slop-vision-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir)?;
    let pattern = dir.join("frame_%04d.jpg");

    // Build a `select` expression that matches our timestamps within ±10ms.
    let select_expr = timestamps_sec
        .iter()
        .map(|t| format!("between(t,{ts:.3},{te:.3})", ts = t - 0.01, te = t + 0.01))
        .collect::<Vec<_>>()
        .join("+");

    let filter = format!(
        "select='{select_expr}',scale={s}:{s}:force_original_aspect_ratio=increase,crop={s}:{s}",
        s = opts.tile_px
    );

    let status = Command::new("ffmpeg")
        .args(["-y", "-i"])
        .arg(input)
        .args([
            "-vf",
            &filter,
            "-vsync",
            "vfr",
            "-q:v",
            &(((100 - opts.jpeg_quality.min(100)) / 5).max(2)).to_string(),
        ])
        .arg(&pattern)
        .status()
        .await
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                TileError::BinaryNotFound
            } else {
                TileError::Io(e)
            }
        })?;
    if !status.success() {
        return Err(TileError::NonZeroExit(status.code().unwrap_or(-1)));
    }

    let mut tiles = Vec::with_capacity(timestamps_sec.len());
    for (i, t) in timestamps_sec.iter().enumerate() {
        let p = dir.join(format!("frame_{:04}.jpg", i + 1));
        if !p.is_file() {
            continue;
        }
        let jpeg = std::fs::read(&p)?;
        tiles.push(FrameTile {
            asset_id: asset_id.to_string(),
            t_sec: *t,
            jpeg,
        });
        let _ = std::fs::remove_file(&p);
    }
    let _ = std::fs::remove_dir(&dir);
    Ok(tiles)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn image_part_uses_data_url() {
        let t = FrameTile {
            asset_id: "a1".into(),
            t_sec: 5.0,
            jpeg: vec![0xff, 0xd8, 0xff],
        };
        let v = t.as_image_part();
        let url = v["image_url"]["url"].as_str().unwrap();
        assert!(url.starts_with("data:image/jpeg;base64,"));
    }

    #[test]
    fn image_part_base64_is_decodable() {
        use base64::Engine;
        let bytes = vec![0xff, 0xd8, 0xff, 0xe0, 0x00, 0x10, 0x4a, 0x46];
        let t = FrameTile {
            asset_id: "a1".into(),
            t_sec: 0.0,
            jpeg: bytes.clone(),
        };
        let v = t.as_image_part();
        let url = v["image_url"]["url"].as_str().unwrap();
        let b64 = url.trim_start_matches("data:image/jpeg;base64,");
        let decoded = base64::engine::general_purpose::STANDARD
            .decode(b64)
            .unwrap();
        assert_eq!(decoded, bytes);
    }

    #[test]
    fn image_part_type_is_image_url() {
        let t = FrameTile {
            asset_id: "a1".into(),
            t_sec: 0.0,
            jpeg: vec![0],
        };
        let v = t.as_image_part();
        assert_eq!(v["type"], "image_url");
    }
}
