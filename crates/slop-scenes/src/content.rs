//! ContentDetector: HSV mean-absolute-difference + fixed threshold.
//!
//! This is the most-used PySceneDetect detector. Tuning notes:
//!
//! - `threshold` is on a 0..=255 scale. The PySceneDetect default is `27.0`
//!   for moderately edited footage; `15.0` is more sensitive (catches softer
//!   cuts and zooms but more false positives), `40.0` is more conservative.
//! - `min_scene_len` is in *decimated* frames. The default of `15` at 5 fps
//!   means a minimum scene length of 3 source-seconds, which is reasonable
//!   for interview / b-roll material.

use crate::detector::{Detector, DetectorOptions, Scene};
use crate::frames::FrameStream;

/// Fixed-threshold content detector.
#[derive(Debug, Clone, Copy)]
pub struct ContentDetector {
    /// Mean absolute HSV difference threshold.
    pub threshold: f32,
    /// Minimum scene length in decimated frames.
    pub min_scene_len: u32,
}

impl Default for ContentDetector {
    fn default() -> Self {
        Self {
            threshold: 27.0,
            min_scene_len: 15,
        }
    }
}

impl Detector for ContentDetector {
    fn detect(&self, stream: &FrameStream, _opts: &DetectorOptions) -> Vec<Scene> {
        let scores = compute_hsv_diff_scores(stream);
        emit_scenes(
            stream,
            &scores,
            |_, score| score > self.threshold,
            self.min_scene_len,
        )
    }
    fn name(&self) -> &'static str {
        "content"
    }
}

/// Convert a sequence of RGB frames into a per-frame HSV mean-absolute-diff
/// score relative to the previous frame. Score for frame 0 is 0.
pub(crate) fn compute_hsv_diff_scores(stream: &FrameStream) -> Vec<f32> {
    let mut scores = Vec::with_capacity(stream.frames.len());
    let mut prev_hsv: Option<Vec<u8>> = None;
    for frame in &stream.frames {
        let hsv = rgb_to_hsv_u8(&frame.data);
        let score = match &prev_hsv {
            Some(prev) => mean_abs_diff(prev, &hsv),
            None => 0.0,
        };
        scores.push(score);
        prev_hsv = Some(hsv);
    }
    scores
}

/// Walk `scores` and emit scene boundaries wherever `is_cut` returns true,
/// subject to `min_scene_len`. Always emits one scene from `0` to the end.
pub(crate) fn emit_scenes<F: Fn(usize, f32) -> bool>(
    stream: &FrameStream,
    scores: &[f32],
    is_cut: F,
    min_scene_len: u32,
) -> Vec<Scene> {
    let total = stream.frames.len();
    if total == 0 {
        return Vec::new();
    }
    let to_sec = |i: usize| -> f64 {
        if stream.fps > 0.0 {
            (i as f64) / stream.fps
        } else {
            0.0
        }
    };

    let mut scenes = Vec::new();
    let mut last_cut: usize = 0;
    for (i, &s) in scores.iter().enumerate() {
        if i == 0 {
            continue;
        }
        if (i - last_cut) as u32 >= min_scene_len && is_cut(i, s) {
            scenes.push(Scene {
                scene_id: format!("shot_{:04}", scenes.len()),
                start_sec: to_sec(last_cut),
                end_sec: to_sec(i),
            });
            last_cut = i;
        }
    }
    scenes.push(Scene {
        scene_id: format!("shot_{:04}", scenes.len()),
        start_sec: to_sec(last_cut),
        end_sec: stream.duration_sec.max(to_sec(total)),
    });
    scenes
}

/// Convert packed RGB888 to packed HSV888 (each component 0..=255).
///
/// We use the same conversion PySceneDetect uses: standard fast integer
/// approximation rather than a colorimetric exact transform. The shape of
/// the difference signal is what matters; absolute color values don't.
pub(crate) fn rgb_to_hsv_u8(rgb: &[u8]) -> Vec<u8> {
    let mut out = vec![0u8; rgb.len()];
    for i in (0..rgb.len()).step_by(3) {
        let r = rgb[i] as i32;
        let g = rgb[i + 1] as i32;
        let b = rgb[i + 2] as i32;
        let max = r.max(g).max(b);
        let min = r.min(g).min(b);
        let delta = max - min;

        let v = max;
        let s = if max == 0 { 0 } else { (delta * 255) / max };
        let h = if delta == 0 {
            0
        } else if max == r {
            // 60 * ((g - b) / delta) on a 0..=360 scale; map to 0..=255.
            let h60 = (60 * (g - b)) / delta.max(1);
            (((h60 + 360) % 360) * 255) / 360
        } else if max == g {
            let h60 = 60 * (b - r) / delta.max(1) + 120;
            (((h60 + 360) % 360) * 255) / 360
        } else {
            let h60 = 60 * (r - g) / delta.max(1) + 240;
            (((h60 + 360) % 360) * 255) / 360
        };
        out[i] = h as u8;
        out[i + 1] = s as u8;
        out[i + 2] = v as u8;
    }
    out
}

pub(crate) fn mean_abs_diff(a: &[u8], b: &[u8]) -> f32 {
    debug_assert_eq!(a.len(), b.len());
    if a.is_empty() {
        return 0.0;
    }
    let mut sum: u64 = 0;
    for i in 0..a.len() {
        let d = (a[i] as i32 - b[i] as i32).abs();
        sum += d as u64;
    }
    sum as f32 / a.len() as f32
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frames::{FrameStream, RgbFrame};

    fn solid_frame(idx: u32, color: [u8; 3], w: u32, h: u32) -> RgbFrame {
        let mut data = vec![0u8; (w * h * 3) as usize];
        for i in (0..data.len()).step_by(3) {
            data[i] = color[0];
            data[i + 1] = color[1];
            data[i + 2] = color[2];
        }
        RgbFrame {
            index: idx,
            width: w,
            height: h,
            data,
        }
    }

    #[test]
    fn solid_color_change_triggers_high_score() {
        let stream = FrameStream {
            frames: vec![
                solid_frame(0, [0, 0, 0], 10, 10),
                solid_frame(1, [0, 0, 0], 10, 10),
                solid_frame(2, [255, 0, 0], 10, 10),
                solid_frame(3, [255, 0, 0], 10, 10),
            ],
            duration_sec: 1.0,
            fps: 4.0,
        };
        let scores = compute_hsv_diff_scores(&stream);
        assert!(scores[2] > scores[1]);
        assert!(scores[2] > 30.0);
    }

    #[test]
    fn detector_emits_at_least_one_scene() {
        let stream = FrameStream {
            frames: (0..30).map(|i| solid_frame(i, [0, 0, 0], 8, 8)).collect(),
            duration_sec: 6.0,
            fps: 5.0,
        };
        let det = ContentDetector::default();
        let scenes = det.detect(&stream, &Default::default());
        assert_eq!(scenes.len(), 1);
        assert!((scenes[0].end_sec - 6.0).abs() < 1e-6);
    }

    #[test]
    fn detector_emits_two_scenes_on_jump() {
        let mut frames: Vec<RgbFrame> = (0..20).map(|i| solid_frame(i, [0, 0, 0], 8, 8)).collect();
        frames.extend((20..40).map(|i| solid_frame(i, [255, 0, 0], 8, 8)));
        let stream = FrameStream {
            frames,
            duration_sec: 8.0,
            fps: 5.0,
        };
        let det = ContentDetector {
            threshold: 27.0,
            min_scene_len: 5,
        };
        let scenes = det.detect(&stream, &Default::default());
        assert!(scenes.len() >= 2);
    }
}
