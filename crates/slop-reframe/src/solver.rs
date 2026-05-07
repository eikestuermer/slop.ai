//! Crop-window solver.

use crate::kalman::Kalman1D;
use crate::yolo::Detection;
use serde::{Deserialize, Serialize};

/// One frame's chosen crop.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct CropFrame {
    /// Time on the source timeline.
    pub t_sec: f64,
    /// Crop x in source pixels.
    pub x: u32,
    /// Crop y.
    pub y: u32,
    /// Crop width.
    pub w: u32,
    /// Crop height.
    pub h: u32,
}

/// Knobs.
#[derive(Debug, Clone, Copy)]
pub struct ReframeOptions {
    /// Source frame width.
    pub src_w: u32,
    /// Source frame height.
    pub src_h: u32,
    /// Target aspect ratio (e.g. `9.0/16.0` for vertical, `1.0` for square).
    pub target_aspect: f32,
    /// Kalman process noise on subject centroid.
    pub q: f32,
    /// Kalman measurement noise.
    pub r: f32,
    /// Maximum pan speed in pixels per source-second (limits jitter).
    pub max_speed_px_per_sec: f32,
}

impl Default for ReframeOptions {
    fn default() -> Self {
        Self {
            src_w: 1920,
            src_h: 1080,
            target_aspect: 9.0 / 16.0,
            q: 0.005,
            r: 0.5,
            max_speed_px_per_sec: 600.0,
        }
    }
}

/// Compute a smooth crop track from a list of per-frame detections.
///
/// `detections_by_frame[i]` is the list of detections for the i-th frame
/// (frames are decimated at `fps`). Empty lists are allowed; the solver
/// holds the last good centroid until a new detection arrives.
pub fn compute_crop_track(
    detections_by_frame: &[Vec<Detection>],
    fps: f64,
    opts: &ReframeOptions,
) -> Vec<CropFrame> {
    if detections_by_frame.is_empty() {
        return Vec::new();
    }

    // Choose subject centroid per frame: the highest-confidence person, or
    // image center if none present.
    let centroids: Vec<(f32, f32)> = detections_by_frame
        .iter()
        .map(|dets| {
            dets.iter()
                .filter(|d| d.class_id == 0)
                .max_by(|a, b| a.score.partial_cmp(&b.score).unwrap())
                .map(|d| (d.cx, d.cy))
                .unwrap_or((0.5, 0.5))
        })
        .collect();

    // Smooth in normalized coordinates.
    let initial = centroids[0];
    let mut kx = Kalman1D::new(initial.0, opts.q, opts.r);
    let mut ky = Kalman1D::new(initial.1, opts.q, opts.r);

    let mut smoothed = Vec::with_capacity(centroids.len());
    let max_step_norm = opts.max_speed_px_per_sec / fps as f32 / opts.src_w.max(opts.src_h) as f32;
    let mut last = initial;
    for c in &centroids {
        let mut nx = kx.step(c.0);
        let mut ny = ky.step(c.1);
        // Clamp pan speed in normalized coords.
        let dx = nx - last.0;
        let dy = ny - last.1;
        let mag = (dx * dx + dy * dy).sqrt();
        if mag > max_step_norm {
            let s = max_step_norm / mag;
            nx = last.0 + dx * s;
            ny = last.1 + dy * s;
        }
        smoothed.push((nx, ny));
        last = (nx, ny);
    }

    // Compute crop window dimensions.
    let src_aspect = opts.src_w as f32 / opts.src_h as f32;
    let (crop_w, crop_h) = if opts.target_aspect < src_aspect {
        // Vertical: full height, narrower width.
        let h = opts.src_h as f32;
        let w = h * opts.target_aspect;
        (w as u32, h as u32)
    } else {
        // Horizontal: full width, shorter height.
        let w = opts.src_w as f32;
        let h = w / opts.target_aspect;
        (w as u32, h as u32)
    };

    smoothed
        .iter()
        .enumerate()
        .map(|(i, (nx, ny))| {
            let cx = (*nx * opts.src_w as f32) as i32;
            let cy = (*ny * opts.src_h as f32) as i32;
            let mut x = cx - (crop_w as i32) / 2;
            let mut y = cy - (crop_h as i32) / 2;
            x = x.clamp(0, (opts.src_w as i32 - crop_w as i32).max(0));
            y = y.clamp(0, (opts.src_h as i32 - crop_h as i32).max(0));
            CropFrame {
                t_sec: i as f64 / fps,
                x: x as u32,
                y: y as u32,
                w: crop_w,
                h: crop_h,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn det(cx: f32, cy: f32) -> Detection {
        Detection {
            cx,
            cy,
            w: 0.1,
            h: 0.2,
            class_id: 0,
            score: 0.9,
        }
    }

    #[test]
    fn vertical_crop_dimensions_correct() {
        let dets = vec![vec![det(0.5, 0.5)]; 30];
        let track = compute_crop_track(
            &dets,
            5.0,
            &ReframeOptions {
                src_w: 1920,
                src_h: 1080,
                target_aspect: 9.0 / 16.0,
                ..Default::default()
            },
        );
        assert_eq!(track.len(), 30);
        // 9:16 of 1080 high -> width = 1080 * 9 / 16 = 607
        assert_eq!(track[0].h, 1080);
        assert!((track[0].w as i32 - 607).abs() < 2);
    }

    #[test]
    fn track_smooths_jumps() {
        let mut dets = Vec::new();
        for i in 0..20 {
            // Subject teleports every other frame; smoother should average.
            let cx = if i % 2 == 0 { 0.3 } else { 0.7 };
            dets.push(vec![det(cx, 0.5)]);
        }
        let track = compute_crop_track(&dets, 5.0, &ReframeOptions::default());
        // Crop-x should not flip every frame.
        let xs: Vec<i32> = track.iter().map(|c| c.x as i32).collect();
        let big_jumps = xs.windows(2).filter(|w| (w[1] - w[0]).abs() > 200).count();
        assert!(big_jumps <= 2, "expected smoothing; got xs={xs:?}");
    }

    #[test]
    fn empty_detections_per_frame_default_to_center() {
        // Every frame has zero detections -> use image center, no panic.
        let dets: Vec<Vec<Detection>> = vec![Vec::new(); 10];
        let track = compute_crop_track(&dets, 5.0, &ReframeOptions::default());
        assert_eq!(track.len(), 10);
        // All crops should be horizontally centered.
        let xs: Vec<u32> = track.iter().map(|c| c.x).collect();
        let max_x = *xs.iter().max().unwrap() as i32;
        let min_x = *xs.iter().min().unwrap() as i32;
        assert!((max_x - min_x) < 50, "centered run shouldn't drift");
    }

    #[test]
    fn empty_input_returns_empty() {
        let track = compute_crop_track(&[], 5.0, &ReframeOptions::default());
        assert!(track.is_empty());
    }

    #[test]
    fn square_aspect_produces_square_crop() {
        let dets = vec![vec![det(0.5, 0.5)]; 5];
        let track = compute_crop_track(
            &dets,
            5.0,
            &ReframeOptions {
                src_w: 1920,
                src_h: 1080,
                target_aspect: 1.0,
                ..Default::default()
            },
        );
        for f in track {
            assert_eq!(f.w, f.h);
        }
    }
}
