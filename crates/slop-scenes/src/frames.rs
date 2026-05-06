//! Decode a decimated RGB stream from a media file via ffmpeg.

use std::path::Path;
use std::process::Stdio;
use thiserror::Error;
use tokio::io::AsyncReadExt;
use tokio::process::Command;

/// A single decoded frame in 8-bit packed RGB.
#[derive(Debug, Clone)]
pub struct RgbFrame {
    /// Frame index in the decimated stream (not the source).
    pub index: u32,
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// Packed RGB888 bytes (`width * height * 3`).
    pub data: Vec<u8>,
}

/// Stream of decimated RGB frames + the rate they were sampled at.
pub struct FrameStream {
    /// All frames, in order.
    pub frames: Vec<RgbFrame>,
    /// Source duration in seconds.
    pub duration_sec: f64,
    /// Sample rate (frames per source second).
    pub fps: f64,
}

/// Errors that can come out of frame decoding.
#[derive(Debug, Error)]
pub enum FrameError {
    /// ffmpeg not on PATH.
    #[error("ffmpeg not found on PATH")]
    BinaryNotFound,
    /// ffmpeg exited non-zero.
    #[error("ffmpeg exited {status}: {stderr}")]
    NonZeroExit {
        /// Exit code.
        status: i32,
        /// Captured stderr.
        stderr: String,
    },
    /// I/O.
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

/// Decode a decimated RGB sequence from `input`. Returns the full sequence.
///
/// `target_fps` is the decimated rate (frames-per-source-second). The
/// PySceneDetect default is 5; we follow it.
///
/// `width` and `height` are the dimensions ffmpeg should rescale to. Smaller
/// is faster; the algorithm is robust to small sizes.
pub async fn decode_decimated_rgb(
    input: impl AsRef<Path>,
    duration_sec: f64,
    target_fps: f64,
    width: u32,
    height: u32,
) -> Result<FrameStream, FrameError> {
    let input = input.as_ref();
    let mut child = Command::new("ffmpeg")
        .args(["-v", "error", "-i"])
        .arg(input)
        .args([
            "-vf",
            &format!("fps={target_fps},scale={width}:{height}"),
            "-f",
            "rawvideo",
            "-pix_fmt",
            "rgb24",
            "pipe:1",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                FrameError::BinaryNotFound
            } else {
                FrameError::Io(e)
            }
        })?;

    let mut stdout = child.stdout.take().expect("stdout piped");
    let frame_size = (width as usize) * (height as usize) * 3;
    let mut buf = Vec::new();
    stdout.read_to_end(&mut buf).await?;
    let status = child.wait().await?;
    if !status.success() {
        let mut stderr = String::new();
        if let Some(mut s) = child.stderr.take() {
            let mut tmp = Vec::new();
            let _ = s.read_to_end(&mut tmp).await;
            stderr = String::from_utf8_lossy(&tmp).to_string();
        }
        return Err(FrameError::NonZeroExit {
            status: status.code().unwrap_or(-1),
            stderr,
        });
    }

    let n = buf.len() / frame_size;
    let mut frames = Vec::with_capacity(n);
    for i in 0..n {
        let start = i * frame_size;
        let end = start + frame_size;
        frames.push(RgbFrame {
            index: i as u32,
            width,
            height,
            data: buf[start..end].to_vec(),
        });
    }

    Ok(FrameStream {
        frames,
        duration_sec,
        fps: target_fps,
    })
}
