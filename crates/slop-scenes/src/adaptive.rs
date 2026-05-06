//! AdaptiveDetector: rolling-mean-relative threshold.
//!
//! The fixed-threshold ContentDetector produces too many false positives on
//! footage with steady motion (camera pans, dolly shots). AdaptiveDetector
//! computes the threshold as `rolling_mean * adaptive_ratio`, so steady
//! motion is absorbed into the baseline and only spikes above it are
//! treated as cuts.

use crate::content::{compute_hsv_diff_scores, emit_scenes};
use crate::detector::{Detector, DetectorOptions, Scene};
use crate::frames::FrameStream;

/// Adaptive content detector.
#[derive(Debug, Clone, Copy)]
pub struct AdaptiveDetector {
    /// Cuts must be `rolling_mean * adaptive_ratio` above the baseline.
    pub adaptive_ratio: f32,
    /// Half-window in frames for the rolling mean.
    pub window: u32,
    /// Minimum scene length in decimated frames.
    pub min_scene_len: u32,
    /// Lower bound on `rolling_mean` to avoid divide-by-tiny on solid clips.
    pub min_threshold: f32,
}

impl Default for AdaptiveDetector {
    fn default() -> Self {
        Self {
            adaptive_ratio: 3.0,
            window: 8,
            min_scene_len: 15,
            min_threshold: 5.0,
        }
    }
}

impl Detector for AdaptiveDetector {
    fn detect(&self, stream: &FrameStream, _opts: &DetectorOptions) -> Vec<Scene> {
        let scores = compute_hsv_diff_scores(stream);
        let n = scores.len();
        let w = self.window as usize;
        let baseline: Vec<f32> = (0..n)
            .map(|i| {
                let lo = i.saturating_sub(w);
                let hi = (i + w + 1).min(n);
                if hi <= lo {
                    return 0.0;
                }
                let count = (hi - lo) as f32;
                let sum: f32 = scores[lo..hi].iter().sum();
                (sum / count).max(self.min_threshold)
            })
            .collect();

        let ratio = self.adaptive_ratio;
        emit_scenes(
            stream,
            &scores,
            |i, score| score > baseline[i] * ratio,
            self.min_scene_len,
        )
    }
    fn name(&self) -> &'static str {
        "adaptive"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frames::{FrameStream, RgbFrame};

    fn frame(idx: u32, c: [u8; 3]) -> RgbFrame {
        let mut data = vec![0u8; 8 * 8 * 3];
        for i in (0..data.len()).step_by(3) {
            data[i] = c[0];
            data[i + 1] = c[1];
            data[i + 2] = c[2];
        }
        RgbFrame {
            index: idx,
            width: 8,
            height: 8,
            data,
        }
    }

    #[test]
    fn adaptive_ignores_steady_motion() {
        // 30 frames where each frame has a small drift. No actual cut.
        let mut frames = Vec::new();
        for i in 0..30 {
            let v = ((i * 5) % 200) as u8;
            frames.push(frame(i, [v, v, v]));
        }
        let stream = FrameStream {
            frames,
            duration_sec: 6.0,
            fps: 5.0,
        };
        let scenes = AdaptiveDetector::default().detect(&stream, &Default::default());
        // We don't assert exactly 1 scene because a drift can still push
        // some pairs over a small adaptive threshold. We do assert that we
        // do not over-segment to the point of one-scene-per-frame.
        assert!(scenes.len() < 5);
    }

    #[test]
    fn adaptive_detects_real_cut() {
        let mut frames: Vec<RgbFrame> = (0..30).map(|i| frame(i, [10, 10, 10])).collect();
        frames.extend((30..60).map(|i| frame(i, [200, 200, 200])));
        let stream = FrameStream {
            frames,
            duration_sec: 12.0,
            fps: 5.0,
        };
        let scenes = AdaptiveDetector::default().detect(&stream, &Default::default());
        assert!(scenes.len() >= 2);
    }
}
