//! Run `ffprobe` and parse the JSON output into a structured [`ProbeResult`].

use crate::error::{MediaError, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;
use tokio::process::Command;

/// Parsed `ffprobe` output, narrowed to the fields Slop AI cares about.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProbeResult {
    /// Duration in seconds.
    pub duration_sec: f64,
    /// True if any stream is `video`.
    pub has_video: bool,
    /// True if any stream is `audio`.
    pub has_audio: bool,
    /// Video frame rate, parsed from `r_frame_rate`.
    pub video_fps: Option<f64>,
    /// Video resolution.
    pub video_resolution: Option<(u32, u32)>,
    /// Audio sample rate.
    pub audio_sample_rate: Option<u32>,
    /// Audio channel count.
    pub audio_channels: Option<u32>,
    /// Container format name, e.g. `mov,mp4,m4a,3gp,3g2,mj2`.
    pub format_name: Option<String>,
}

impl ProbeResult {
    /// Convert to a `slop_core::Asset`. Caller supplies the asset id and uri.
    pub fn into_asset(self, asset_id: String, uri: String) -> slop_core::Asset {
        slop_core::Asset {
            asset_id,
            uri,
            duration_sec: self.duration_sec,
            has_video: self.has_video,
            has_audio: self.has_audio,
            fps: self.video_fps,
            resolution: self.video_resolution.map(|(w, h)| slop_core::Resolution { w, h }),
            transcript_ref: None,
            shot_list_ref: None,
        }
    }
}

/// Probe a media file with `ffprobe -v quiet -print_format json -show_format -show_streams`.
pub async fn probe_asset(path: impl AsRef<Path>) -> Result<ProbeResult> {
    let path = path.as_ref();
    let output = Command::new("ffprobe")
        .args([
            "-v",
            "quiet",
            "-print_format",
            "json",
            "-show_format",
            "-show_streams",
        ])
        .arg(path)
        .output()
        .await
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                MediaError::BinaryNotFound("ffprobe")
            } else {
                MediaError::Io(e)
            }
        })?;

    if !output.status.success() {
        return Err(MediaError::NonZeroExit {
            binary: "ffprobe",
            status: output.status.code().unwrap_or(-1),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        });
    }

    let json: serde_json::Value = serde_json::from_slice(&output.stdout)?;
    parse_probe_json(&json)
}

/// Parse a `ffprobe` JSON document. Public so unit tests can call it without
/// invoking the binary.
pub fn parse_probe_json(json: &serde_json::Value) -> Result<ProbeResult> {
    let format = json.get("format");
    let streams = json
        .get("streams")
        .and_then(|s| s.as_array())
        .ok_or_else(|| MediaError::ParseFailure {
            binary: "ffprobe",
            message: "missing streams".into(),
        })?;

    let duration_sec = format
        .and_then(|f| f.get("duration"))
        .and_then(|v| v.as_str())
        .and_then(|s| s.parse::<f64>().ok())
        .or_else(|| {
            // Some MP4s only carry duration in stream metadata.
            streams.iter().find_map(|s| {
                s.get("duration")
                    .and_then(|v| v.as_str())
                    .and_then(|s| s.parse::<f64>().ok())
            })
        })
        .unwrap_or(0.0);

    let format_name = format
        .and_then(|f| f.get("format_name"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let mut has_video = false;
    let mut has_audio = false;
    let mut video_fps = None;
    let mut video_resolution = None;
    let mut audio_sample_rate = None;
    let mut audio_channels = None;

    for s in streams {
        let codec_type = s.get("codec_type").and_then(|v| v.as_str());
        match codec_type {
            Some("video") => {
                has_video = true;
                if video_fps.is_none() {
                    video_fps = s
                        .get("r_frame_rate")
                        .and_then(|v| v.as_str())
                        .and_then(parse_rational);
                }
                if video_resolution.is_none() {
                    let w = s.get("width").and_then(|v| v.as_u64()).map(|v| v as u32);
                    let h = s.get("height").and_then(|v| v.as_u64()).map(|v| v as u32);
                    if let (Some(w), Some(h)) = (w, h) {
                        video_resolution = Some((w, h));
                    }
                }
            }
            Some("audio") => {
                has_audio = true;
                if audio_sample_rate.is_none() {
                    audio_sample_rate = s
                        .get("sample_rate")
                        .and_then(|v| v.as_str())
                        .and_then(|s| s.parse::<u32>().ok());
                }
                if audio_channels.is_none() {
                    audio_channels = s.get("channels").and_then(|v| v.as_u64()).map(|v| v as u32);
                }
            }
            _ => {}
        }
    }

    Ok(ProbeResult {
        duration_sec,
        has_video,
        has_audio,
        video_fps,
        video_resolution,
        audio_sample_rate,
        audio_channels,
        format_name,
    })
}

fn parse_rational(s: &str) -> Option<f64> {
    let mut parts = s.splitn(2, '/');
    let num: f64 = parts.next()?.parse().ok()?;
    let den: f64 = parts.next().unwrap_or("1").parse().ok()?;
    if den == 0.0 {
        None
    } else {
        Some(num / den)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_typical_ffprobe_json() {
        let raw = serde_json::json!({
            "format": {
                "format_name": "mov,mp4",
                "duration": "183.42"
            },
            "streams": [
                {
                    "codec_type": "video",
                    "width": 1920,
                    "height": 1080,
                    "r_frame_rate": "30000/1001"
                },
                {
                    "codec_type": "audio",
                    "sample_rate": "48000",
                    "channels": 2
                }
            ]
        });
        let r = parse_probe_json(&raw).unwrap();
        assert!((r.duration_sec - 183.42).abs() < 1e-3);
        assert!(r.has_video && r.has_audio);
        assert_eq!(r.video_resolution, Some((1920, 1080)));
        assert!((r.video_fps.unwrap() - 29.97).abs() < 0.05);
        assert_eq!(r.audio_sample_rate, Some(48_000));
        assert_eq!(r.audio_channels, Some(2));
    }

    #[test]
    fn parses_audio_only_file() {
        let raw = serde_json::json!({
            "format": { "duration": "60.0", "format_name": "wav" },
            "streams": [
                { "codec_type": "audio", "sample_rate": "44100", "channels": 1 }
            ]
        });
        let r = parse_probe_json(&raw).unwrap();
        assert!(!r.has_video && r.has_audio);
        assert_eq!(r.audio_sample_rate, Some(44_100));
    }
}
