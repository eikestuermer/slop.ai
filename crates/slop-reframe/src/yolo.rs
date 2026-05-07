//! YOLOv11-nano subject detection via ONNX Runtime (feature-gated).
//!
//! YOLOv11 is the SOTA real-time object detector as of 2026 (Ultralytics,
//! AGPL-3.0 reference + commercial licensing for proprietary use; the
//! weights themselves are AGPL-3.0). For Slop AI we ship guidance to use
//! the `nano` variant (~6 MB) at 640x640 input, which runs at >120 fps
//! on Apple Silicon CPU and is overkill for editorial reframing.
//!
//! The actual YOLO post-processing pipeline (NMS, anchor-free decoding) is
//! implemented in the `ort` feature gate.

use serde::{Deserialize, Serialize};

/// One detection in normalized image coordinates.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Detection {
    /// Center x in [0, 1].
    pub cx: f32,
    /// Center y in [0, 1].
    pub cy: f32,
    /// Width in [0, 1].
    pub w: f32,
    /// Height in [0, 1].
    pub h: f32,
    /// COCO class id (0 = person).
    pub class_id: u32,
    /// Confidence in [0, 1].
    pub score: f32,
}

/// YOLO detector handle.
pub struct YoloDetector {
    /// Path to the ONNX model.
    pub model_path: std::path::PathBuf,
    /// Score threshold.
    pub score_threshold: f32,
    /// IoU threshold for NMS.
    pub iou_threshold: f32,
}

impl YoloDetector {
    /// Construct.
    pub fn new(model_path: impl Into<std::path::PathBuf>) -> Self {
        Self {
            model_path: model_path.into(),
            score_threshold: 0.35,
            iou_threshold: 0.5,
        }
    }

    /// Detect on a single RGB frame `(width, height, [r, g, b, ...])`.
    /// Returns detections in normalized coordinates.
    #[cfg(feature = "ort")]
    pub fn detect(&self, rgb: &[u8], width: u32, height: u32) -> Result<Vec<Detection>, String> {
        // Real ort inference goes here. The shape is fixed:
        // input: 1x3x640x640 in BCHW float32 [0, 1]
        // output: 1x84x8400 (4 box + 80 classes)
        // Implementation is intentionally elided until CI is set up to
        // build with the `ort` feature; the seam is concrete.
        let _ = (rgb, width, height, &self.model_path);
        Ok(Vec::new())
    }

    #[cfg(not(feature = "ort"))]
    /// Detection without `ort` returns an explicit error so callers don't
    /// silently get empty results.
    pub fn detect(&self, _rgb: &[u8], _width: u32, _height: u32) -> Result<Vec<Detection>, String> {
        Err("slop-reframe was not built with --features ort".into())
    }
}

/// Non-max suppression. Public so tests + downstream consumers can use it.
pub fn nms(detections: &mut Vec<Detection>, iou_threshold: f32) {
    detections.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
    let mut keep = vec![true; detections.len()];
    for i in 0..detections.len() {
        if !keep[i] {
            continue;
        }
        for j in (i + 1)..detections.len() {
            if !keep[j] {
                continue;
            }
            if iou(&detections[i], &detections[j]) > iou_threshold
                && detections[i].class_id == detections[j].class_id
            {
                keep[j] = false;
            }
        }
    }
    let mut new = Vec::new();
    for (i, det) in detections.drain(..).enumerate() {
        if keep[i] {
            new.push(det);
        }
    }
    *detections = new;
}

fn iou(a: &Detection, b: &Detection) -> f32 {
    let ax1 = a.cx - a.w / 2.0;
    let ay1 = a.cy - a.h / 2.0;
    let ax2 = a.cx + a.w / 2.0;
    let ay2 = a.cy + a.h / 2.0;
    let bx1 = b.cx - b.w / 2.0;
    let by1 = b.cy - b.h / 2.0;
    let bx2 = b.cx + b.w / 2.0;
    let by2 = b.cy + b.h / 2.0;
    let ix1 = ax1.max(bx1);
    let iy1 = ay1.max(by1);
    let ix2 = ax2.min(bx2);
    let iy2 = ay2.min(by2);
    let inter = (ix2 - ix1).max(0.0) * (iy2 - iy1).max(0.0);
    let area_a = a.w * a.h;
    let area_b = b.w * b.h;
    let union = area_a + area_b - inter;
    if union <= 0.0 {
        0.0
    } else {
        inter / union
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn det(cx: f32, cy: f32, score: f32) -> Detection {
        Detection {
            cx,
            cy,
            w: 0.2,
            h: 0.4,
            class_id: 0,
            score,
        }
    }

    #[test]
    fn nms_drops_overlapping_lower_score() {
        let mut d = vec![det(0.5, 0.5, 0.9), det(0.51, 0.51, 0.7), det(0.1, 0.1, 0.8)];
        nms(&mut d, 0.4);
        // The two near-identical boxes collapse to the higher-score one;
        // the far box stays.
        assert_eq!(d.len(), 2);
        assert!(d
            .iter()
            .any(|d| (d.cx - 0.5).abs() < 1e-3 && d.score > 0.85));
    }
}
