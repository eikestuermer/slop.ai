//! 1-D Kalman filter for smoothing the subject centroid track.

use serde::{Deserialize, Serialize};

/// Constant-velocity Kalman filter.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Kalman1D {
    /// State: `[position, velocity]`.
    pub x: [f32; 2],
    /// Covariance.
    pub p: [[f32; 2]; 2],
    /// Process noise.
    pub q: f32,
    /// Measurement noise.
    pub r: f32,
}

impl Kalman1D {
    /// Construct with initial position.
    pub fn new(initial_pos: f32, q: f32, r: f32) -> Self {
        Self {
            x: [initial_pos, 0.0],
            p: [[1.0, 0.0], [0.0, 1.0]],
            q,
            r,
        }
    }

    /// Advance one step (assumes `dt = 1`).
    pub fn step(&mut self, measurement: f32) -> f32 {
        // Predict.
        let dt = 1.0;
        let x_pred = [self.x[0] + dt * self.x[1], self.x[1]];
        let p_pred = [
            [
                self.p[0][0] + dt * (self.p[1][0] + self.p[0][1]) + dt * dt * self.p[1][1] + self.q,
                self.p[0][1] + dt * self.p[1][1],
            ],
            [self.p[1][0] + dt * self.p[1][1], self.p[1][1] + self.q],
        ];
        // Update.
        let s = p_pred[0][0] + self.r;
        let k0 = p_pred[0][0] / s;
        let k1 = p_pred[1][0] / s;
        let y = measurement - x_pred[0];
        self.x = [x_pred[0] + k0 * y, x_pred[1] + k1 * y];
        self.p = [
            [(1.0 - k0) * p_pred[0][0], (1.0 - k0) * p_pred[0][1]],
            [
                -k1 * p_pred[0][0] + p_pred[1][0],
                -k1 * p_pred[0][1] + p_pred[1][1],
            ],
        ];
        self.x[0]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kalman_tracks_a_step() {
        let mut k = Kalman1D::new(0.0, 0.01, 0.5);
        for _ in 0..50 {
            k.step(10.0);
        }
        assert!((k.x[0] - 10.0).abs() < 0.5);
    }

    #[test]
    fn kalman_smooths_noise() {
        let mut k = Kalman1D::new(0.0, 0.01, 1.0);
        let target = 5.0_f32;
        let mut last = 0.0;
        for i in 0..100 {
            let noise = if i % 2 == 0 { 1.0 } else { -1.0 };
            last = k.step(target + noise);
        }
        // After noise averaging the estimate should sit near target.
        assert!((last - target).abs() < 1.0);
    }
}
