//! FFmpeg `loudnorm` filter emission.
//!
//! BS.1770-4 normalization is a two-pass operation: measure first, then
//! apply with the measured values baked in. Emitting only the apply pass
//! relies on ffmpeg's runtime measurement (single-pass linear mode), which
//! is acceptable for preview but not for delivery. For final renders the
//! caller should run a measurement pass first via
//! `loudnorm=...:print_format=json` and pass the parsed values back here.

use crate::loudness::{LoudnessMetrics, LoudnessTarget};

/// Emit a `loudnorm` filter fragment for the apply pass.
pub fn loudnorm_filtergraph(
    input: &str,
    output: &str,
    target: LoudnessTarget,
    measured: Option<LoudnessMetrics>,
) -> String {
    match measured {
        Some(m) => format!(
            "[{input}]loudnorm=I={i:.2}:TP={tp:.2}:LRA={lra:.2}:measured_I={mi:.2}:measured_LRA={mlra:.2}:measured_TP={mtp:.2}:measured_thresh=-70:linear=true[{output}];",
            input = input,
            output = output,
            i = target.lufs,
            tp = target.true_peak_dbfs,
            lra = target.lra,
            mi = m.integrated_lufs,
            mlra = m.lra,
            mtp = m.true_peak_dbtp,
        ),
        None => format!(
            "[{input}]loudnorm=I={i:.2}:TP={tp:.2}:LRA={lra:.2}:linear=true[{output}];",
            input = input,
            output = output,
            i = target.lufs,
            tp = target.true_peak_dbfs,
            lra = target.lra,
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_pass_filtergraph_uses_target_values() {
        let g = loudnorm_filtergraph("a0", "anorm", LoudnessTarget::STREAMING, None);
        assert!(g.contains("I=-14.00"));
        assert!(g.contains("TP=-1.00"));
        assert!(g.contains("[a0]"));
        assert!(g.contains("[anorm]"));
    }

    #[test]
    fn two_pass_includes_measured_values() {
        let m = LoudnessMetrics {
            integrated_lufs: -18.4,
            lra: 8.2,
            true_peak_dbtp: -2.3,
        };
        let g = loudnorm_filtergraph("a0", "anorm", LoudnessTarget::BROADCAST, Some(m));
        assert!(g.contains("measured_I=-18.40"));
        assert!(g.contains("measured_LRA=8.20"));
        assert!(g.contains("measured_TP=-2.30"));
    }
}
