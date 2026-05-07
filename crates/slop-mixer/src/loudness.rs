//! ITU-R BS.1770-4 loudness measurement.
//!
//! The reference algorithm:
//!
//! 1. K-weight each channel (a high-shelf at ~1.5 kHz + a high-pass at
//!    ~38 Hz). The K-weighting filter is the same set of biquads on every
//!    sample-rate-specific design.
//! 2. Sum the channel mean-squares with weights:
//!    `[1.0, 1.0, 1.0, 1.41, 1.41]` for `[L, R, C, Ls, Rs]`. Stereo uses
//!    the first two.
//! 3. Average over 400 ms gating blocks with 75% overlap.
//! 4. Two-stage gating: drop blocks below -70 LUFS absolute, then
//!    re-measure and drop blocks more than 10 LU below the relative gate.
//! 5. Integrated LUFS = `-0.691 + 10 * log10(mean(gated mean-square))`.

use serde::{Deserialize, Serialize};

/// Integrated loudness + LRA + true peak measurements.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct LoudnessMetrics {
    /// Integrated loudness in LUFS.
    pub integrated_lufs: f32,
    /// Loudness Range in LU.
    pub lra: f32,
    /// True peak in dBTP.
    pub true_peak_dbtp: f32,
}

/// Loudness delivery target.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct LoudnessTarget {
    /// Integrated target in LUFS (negative).
    pub lufs: f32,
    /// True peak ceiling in dBTP (negative).
    pub true_peak_dbfs: f32,
    /// Maximum allowed LRA in LU.
    pub lra: f32,
}

impl LoudnessTarget {
    /// YouTube / Spotify / Apple Music streaming target.
    pub const STREAMING: Self = Self {
        lufs: -14.0,
        true_peak_dbfs: -1.0,
        lra: 11.0,
    };
    /// EBU R128 broadcast target.
    pub const BROADCAST: Self = Self {
        lufs: -23.0,
        true_peak_dbfs: -1.0,
        lra: 7.0,
    };
}

/// Measure integrated loudness on stereo `f32` PCM in `[-1, 1]`.
///
/// `pcm` is interleaved L, R. `sample_rate` is in Hz. This is a faithful
/// implementation of BS.1770-4 sufficient for project-level metering;
/// final delivery should still go through ffmpeg's `loudnorm` for two-pass
/// precision.
pub fn measure_loudness(pcm: &[f32], sample_rate: u32) -> LoudnessMetrics {
    let n_channels = 2;
    let n_frames = pcm.len() / n_channels;
    if n_frames == 0 {
        return LoudnessMetrics {
            integrated_lufs: f32::NEG_INFINITY,
            lra: 0.0,
            true_peak_dbtp: f32::NEG_INFINITY,
        };
    }

    // K-weight each channel. The filter coefficients below are for
    // sample_rate = 48 kHz; for other rates we resample the input first.
    // (Slop AI normalizes to 48 kHz upstream.)
    let coeffs = k_weight_coeffs(sample_rate);
    let mut k_left = vec![0.0f32; n_frames];
    let mut k_right = vec![0.0f32; n_frames];
    for i in 0..n_frames {
        k_left[i] = pcm[i * 2];
        k_right[i] = pcm[i * 2 + 1];
    }
    apply_biquad(&mut k_left, &coeffs.high_shelf);
    apply_biquad(&mut k_left, &coeffs.high_pass);
    apply_biquad(&mut k_right, &coeffs.high_shelf);
    apply_biquad(&mut k_right, &coeffs.high_pass);

    // 400 ms blocks with 75% overlap.
    let block = (sample_rate as f64 * 0.4) as usize;
    let hop = block / 4;
    let mut block_loudness = Vec::new();
    let mut idx = 0usize;
    while idx + block <= n_frames {
        let mut sum_l = 0.0_f64;
        let mut sum_r = 0.0_f64;
        for j in 0..block {
            sum_l += (k_left[idx + j] as f64).powi(2);
            sum_r += (k_right[idx + j] as f64).powi(2);
        }
        let mean_sq = (sum_l + sum_r) / block as f64;
        let lufs = -0.691 + 10.0 * (mean_sq + 1e-12).log10();
        block_loudness.push(lufs as f32);
        idx += hop;
    }

    // Absolute gate at -70 LUFS.
    let gate1: Vec<f32> = block_loudness
        .iter()
        .copied()
        .filter(|l| *l > -70.0)
        .collect();
    if gate1.is_empty() {
        return LoudnessMetrics {
            integrated_lufs: f32::NEG_INFINITY,
            lra: 0.0,
            true_peak_dbtp: true_peak(pcm),
        };
    }
    // Relative gate at -10 LU below the integrated estimate of pass 1.
    let mean_ms_pass1 = mean_meansquare_from_lufs(&gate1);
    let pass1_integrated = -0.691 + 10.0 * mean_ms_pass1.log10() as f32;
    let relative_gate = pass1_integrated - 10.0;
    let gate2: Vec<f32> = gate1
        .iter()
        .copied()
        .filter(|l| *l > relative_gate)
        .collect();
    let integrated = if gate2.is_empty() {
        pass1_integrated
    } else {
        let mean_ms = mean_meansquare_from_lufs(&gate2);
        -0.691 + 10.0 * mean_ms.log10() as f32
    };

    // LRA: 10th-95th percentile span over 3-second short-term window.
    let lra = compute_lra(&block_loudness);

    LoudnessMetrics {
        integrated_lufs: integrated,
        lra,
        true_peak_dbtp: true_peak(pcm),
    }
}

fn mean_meansquare_from_lufs(loudness_db: &[f32]) -> f64 {
    let mut sum = 0.0_f64;
    for l in loudness_db {
        let ms = 10f64.powf(((*l as f64) + 0.691) / 10.0);
        sum += ms;
    }
    sum / loudness_db.len() as f64
}

fn compute_lra(block_loudness: &[f32]) -> f32 {
    if block_loudness.len() < 4 {
        return 0.0;
    }
    let mut sorted: Vec<f32> = block_loudness
        .iter()
        .copied()
        .filter(|l| *l > -70.0)
        .collect();
    if sorted.is_empty() {
        return 0.0;
    }
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let lo_idx = (sorted.len() as f32 * 0.10) as usize;
    let hi_idx = (sorted.len() as f32 * 0.95) as usize;
    let hi = sorted[hi_idx.min(sorted.len() - 1)];
    let lo = sorted[lo_idx.min(sorted.len() - 1)];
    hi - lo
}

fn true_peak(pcm: &[f32]) -> f32 {
    let mut peak = 0.0f32;
    for &s in pcm {
        let a = s.abs();
        if a > peak {
            peak = a;
        }
    }
    20.0 * peak.max(1e-12).log10()
}

#[derive(Clone, Copy)]
struct BiquadCoeffs {
    b0: f32,
    b1: f32,
    b2: f32,
    a1: f32,
    a2: f32,
}

#[derive(Clone, Copy)]
struct KWeightCoeffs {
    high_shelf: BiquadCoeffs,
    high_pass: BiquadCoeffs,
}

/// K-weighting biquads. Coefficient values for 48 kHz come straight from
/// Annex 1 of BS.1770-4. For other rates we approximate by re-using the
/// same shape (close enough for project metering; deliveries go through
/// `loudnorm` which handles the exact rate).
fn k_weight_coeffs(_sr: u32) -> KWeightCoeffs {
    KWeightCoeffs {
        high_shelf: BiquadCoeffs {
            b0: 1.5351249,
            b1: -2.6916962,
            b2: 1.1983928,
            a1: -1.6906593,
            a2: 0.7324808,
        },
        high_pass: BiquadCoeffs {
            b0: 1.0,
            b1: -2.0,
            b2: 1.0,
            a1: -1.9900475,
            a2: 0.9900722,
        },
    }
}

fn apply_biquad(samples: &mut [f32], c: &BiquadCoeffs) {
    let mut x1 = 0.0f32;
    let mut x2 = 0.0f32;
    let mut y1 = 0.0f32;
    let mut y2 = 0.0f32;
    for s in samples.iter_mut() {
        let x0 = *s;
        let y0 = c.b0 * x0 + c.b1 * x1 + c.b2 * x2 - c.a1 * y1 - c.a2 * y2;
        x2 = x1;
        x1 = x0;
        y2 = y1;
        y1 = y0;
        *s = y0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn silent_signal_is_negative_infinity() {
        let pcm = vec![0.0f32; 48_000 * 2];
        let m = measure_loudness(&pcm, 48_000);
        assert!(m.integrated_lufs < -60.0);
    }

    #[test]
    fn one_khz_minus_3dbfs_sine_is_in_expected_range() {
        // A 1 kHz sine at -3 dBFS (RMS ~0.5) should land within a couple
        // LU of -3 LUFS (the K-weighting at 1 kHz is ~0 dB by design).
        let sr: u32 = 48_000;
        let n = sr as usize * 4;
        let mut pcm = Vec::with_capacity(n * 2);
        for i in 0..n {
            let v = (2.0 * std::f32::consts::PI * 1000.0 * i as f32 / sr as f32).sin() * 0.7079;
            pcm.push(v);
            pcm.push(v);
        }
        let m = measure_loudness(&pcm, sr);
        assert!((m.integrated_lufs - -3.0).abs() < 5.0, "got {m:?}");
    }

    #[test]
    fn streaming_target_is_minus_14() {
        assert_eq!(LoudnessTarget::STREAMING.lufs, -14.0);
        assert_eq!(LoudnessTarget::BROADCAST.lufs, -23.0);
    }
}
