//! Compute a downsampled waveform peak array for the timeline canvas.
//!
//! The flow:
//! 1. ask `ffmpeg` to write a mono 16-bit PCM WAV to a temp file at a low
//!    sample rate (default 16 kHz),
//! 2. read the WAV with `hound`,
//! 3. bucket the samples into `n_buckets` and emit (min, max) per bucket.
//!
//! For long files this is still fast because the WAV is mono and 16 kHz.

use crate::error::{MediaError, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;
use tokio::process::Command;

/// Options for waveform extraction.
#[derive(Debug, Clone)]
pub struct WaveformOptions {
    /// Number of (min, max) pairs to produce.
    pub n_buckets: u32,
    /// Sample rate to decode to before bucketing.
    pub sample_rate: u32,
}

impl Default for WaveformOptions {
    fn default() -> Self {
        Self {
            n_buckets: 2048,
            sample_rate: 16_000,
        }
    }
}

/// Output of [`generate_waveform_peaks`].
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WaveformPeaks {
    /// `2 * n_buckets` floats in `[-1, 1]`: `[min0, max0, min1, max1, ...]`.
    pub peaks: Vec<f32>,
    /// Number of (min, max) pairs.
    pub n_buckets: u32,
    /// Total source samples used.
    pub n_samples: u64,
}

/// Generate a peak array for `input`.
pub async fn generate_waveform_peaks(
    input: impl AsRef<Path>,
    opts: &WaveformOptions,
) -> Result<WaveformPeaks> {
    let input = input.as_ref();
    let tmp = std::env::temp_dir().join(format!("slop-waveform-{}.wav", slop_core::ids::asset()));

    let status = Command::new("ffmpeg")
        .args(["-y", "-i"])
        .arg(input)
        .args([
            "-vn",
            "-ac",
            "1",
            "-ar",
            &opts.sample_rate.to_string(),
            "-f",
            "wav",
            "-acodec",
            "pcm_s16le",
        ])
        .arg(&tmp)
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
        let _ = tokio::fs::remove_file(&tmp).await;
        return Err(MediaError::NonZeroExit {
            binary: "ffmpeg",
            status: status.code().unwrap_or(-1),
            stderr: String::new(),
        });
    }

    let peaks = bucket_samples(&tmp, opts.n_buckets)?;
    let _ = tokio::fs::remove_file(&tmp).await;
    Ok(peaks)
}

/// Read a mono 16-bit PCM WAV and bucket into `n_buckets` (min, max) pairs.
/// Public for testing.
pub fn bucket_samples(wav: &Path, n_buckets: u32) -> Result<WaveformPeaks> {
    let mut reader = hound::WavReader::open(wav)?;
    let total = reader.duration() as u64;
    let bucket_size = (total as f64 / n_buckets as f64).max(1.0);
    let mut peaks = Vec::with_capacity((n_buckets as usize) * 2);

    let mut samples = reader.samples::<i16>();
    for b in 0..n_buckets {
        let start = (b as f64 * bucket_size) as u64;
        let end = (((b + 1) as f64 * bucket_size) as u64).min(total);
        let mut min: i32 = i16::MAX as i32;
        let mut max: i32 = i16::MIN as i32;
        for _ in start..end {
            let s = match samples.next() {
                Some(Ok(v)) => v as i32,
                _ => break,
            };
            if s < min {
                min = s;
            }
            if s > max {
                max = s;
            }
        }
        if max < min {
            // Empty bucket.
            min = 0;
            max = 0;
        }
        peaks.push(min as f32 / i16::MAX as f32);
        peaks.push(max as f32 / i16::MAX as f32);
    }

    Ok(WaveformPeaks {
        peaks,
        n_buckets,
        n_samples: total,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bucketing_handles_tiny_input() {
        // Synthesize a tiny WAV with hound and verify bucketing.
        let dir = std::env::temp_dir().join(format!("slop-wav-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("tiny.wav");
        let spec = hound::WavSpec {
            channels: 1,
            sample_rate: 16_000,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        let mut writer = hound::WavWriter::create(&path, spec).unwrap();
        for i in 0..10_000 {
            let v = ((i as f32 * 0.01).sin() * (i16::MAX as f32 * 0.5)) as i16;
            writer.write_sample(v).unwrap();
        }
        writer.finalize().unwrap();

        let peaks = bucket_samples(&path, 100).unwrap();
        assert_eq!(peaks.n_buckets, 100);
        assert_eq!(peaks.peaks.len(), 200);
        assert!(peaks.peaks.iter().all(|v| v.is_finite()));
    }
}
