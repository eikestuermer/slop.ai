//! Audio cross-correlation sync.

use realfft::RealFftPlanner;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Per-angle sync result.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SyncResult {
    /// Angle id (caller-supplied stable identifier).
    pub angle_id: String,
    /// Number of seconds this angle should be shifted on the multicam
    /// timeline. Positive = this angle starts later than the reference.
    pub offset_sec: f64,
    /// Peak correlation in [0, 1]. Below ~0.3 the sync is unreliable.
    pub confidence: f32,
}

/// Errors during sync.
#[derive(Debug, Error)]
pub enum SyncError {
    /// Empty input.
    #[error("at least 2 angles required")]
    NotEnoughAngles,
    /// Audio length mismatch (sample-rate mixup).
    #[error("audio length mismatch")]
    LengthMismatch,
    /// FFT planning failure (out of memory, illegal length).
    #[error("fft: {0}")]
    Fft(String),
}

/// Compute sync offsets between angles relative to the reference (the
/// longest input). All inputs must share the same sample rate.
pub fn compute_sync_offsets(
    angles: &[(String, Vec<f32>)],
    sample_rate: u32,
) -> Result<Vec<SyncResult>, SyncError> {
    if angles.len() < 2 {
        return Err(SyncError::NotEnoughAngles);
    }
    // Pick the longest as reference. When lengths tie, prefer the lowest
    // index (so the caller-provided ordering becomes the tiebreaker).
    let ref_idx = angles
        .iter()
        .enumerate()
        .fold(0usize, |best, (i, (_, pcm))| {
            if pcm.len() > angles[best].1.len() {
                i
            } else {
                best
            }
        });
    let reference = &angles[ref_idx].1;

    let mut out = Vec::with_capacity(angles.len());
    for (i, (id, pcm)) in angles.iter().enumerate() {
        if i == ref_idx {
            out.push(SyncResult {
                angle_id: id.clone(),
                offset_sec: 0.0,
                confidence: 1.0,
            });
            continue;
        }
        let (lag_samples, peak) = xcorr_argmax(reference, pcm)?;
        out.push(SyncResult {
            angle_id: id.clone(),
            offset_sec: lag_samples as f64 / sample_rate as f64,
            confidence: peak,
        });
    }
    Ok(out)
}

/// FFT-based cross-correlation: lag = argmax over (-N, +N) lags.
///
/// Returns `(lag_in_samples, peak_correlation_in_0_to_1)`.
fn xcorr_argmax(a: &[f32], b: &[f32]) -> Result<(i64, f32), SyncError> {
    if a.is_empty() || b.is_empty() {
        return Err(SyncError::LengthMismatch);
    }
    let n = a.len() + b.len();
    let n = n.next_power_of_two();
    let mut planner = RealFftPlanner::<f32>::new();
    let r2c = planner.plan_fft_forward(n);
    let c2r = planner.plan_fft_inverse(n);

    let mut a_padded = vec![0.0f32; n];
    let mut b_padded = vec![0.0f32; n];
    a_padded[..a.len()].copy_from_slice(a);
    b_padded[..b.len()].copy_from_slice(b);

    let mut a_spec = r2c.make_output_vec();
    let mut b_spec = r2c.make_output_vec();
    r2c.process(&mut a_padded, &mut a_spec)
        .map_err(|e| SyncError::Fft(e.to_string()))?;
    r2c.process(&mut b_padded, &mut b_spec)
        .map_err(|e| SyncError::Fft(e.to_string()))?;

    // Cross-correlation in frequency domain.
    // Convention: c[k] = sum_t a[t] * b[t+k].
    // c[k] is the IFFT of B * conj(A); peak at k=+d when b is a delayed by d.
    let mut prod_spec = vec![realfft::num_complex::Complex::<f32>::new(0.0, 0.0); a_spec.len()];
    for i in 0..a_spec.len() {
        prod_spec[i] = b_spec[i] * a_spec[i].conj();
    }

    let mut corr = vec![0.0f32; n];
    c2r.process(&mut prod_spec, &mut corr)
        .map_err(|e| SyncError::Fft(e.to_string()))?;

    // Find the argmax.
    let mut best_idx = 0usize;
    let mut best_val = f32::NEG_INFINITY;
    for (i, &v) in corr.iter().enumerate() {
        if v > best_val {
            best_val = v;
            best_idx = i;
        }
    }

    // Map argmax to signed lag: indices > N/2 are negative lags.
    let lag = if best_idx > n / 2 {
        best_idx as i64 - n as i64
    } else {
        best_idx as i64
    };

    // Normalize peak by sqrt(energy(a) * energy(b)) for [0, 1] confidence.
    let ea: f32 = a.iter().map(|s| s * s).sum();
    let eb: f32 = b.iter().map(|s| s * s).sum();
    let denom = (ea * eb).sqrt().max(1e-12);
    let conf = (best_val / (denom * n as f32)).clamp(0.0, 1.0);

    Ok((lag, conf))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn synth_signal(len: usize, freq: f32, sr: u32) -> Vec<f32> {
        // A sine modulated with a slow gaussian envelope so the
        // cross-correlation has an unambiguous peak (a pure sine has
        // multiple equally-good peaks, one per period).
        let center = len as f32 / 2.0;
        let sigma = len as f32 / 6.0;
        (0..len)
            .map(|i| {
                let env = (-((i as f32 - center).powi(2)) / (2.0 * sigma * sigma)).exp();
                (2.0 * std::f32::consts::PI * freq * i as f32 / sr as f32).sin() * 0.5 * env
            })
            .collect()
    }

    #[test]
    fn detects_known_offset() {
        let sr = 16_000;
        let n = sr as usize * 4;
        let base = synth_signal(n, 440.0, sr);
        // Shift by 2000 samples (0.125s).
        let mut shifted = vec![0.0; 2000];
        shifted.extend_from_slice(&base[..n - 2000]);

        let res = compute_sync_offsets(
            &[("ref".into(), base.clone()), ("late".into(), shifted)],
            sr,
        )
        .unwrap();
        let late = res.iter().find(|r| r.angle_id == "late").unwrap();
        assert!((late.offset_sec - 0.125).abs() < 0.01, "got {late:?}");
        assert!(late.confidence > 0.5);
    }
}
