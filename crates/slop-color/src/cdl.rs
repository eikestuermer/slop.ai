//! ASC CDL primary correction.
//!
//! Per the spec: `output = pow(max(0, input * slope + offset), power)`
//! followed by a global saturation adjustment around the luminance axis.
//! Slop AI exposes this in user-friendly Resolve-style terms:
//!
//! - lift   == offset (when slope=1)
//! - gain   == slope
//! - gamma  == 1/power
//!
//! All channels (R, G, B, master) get independent (slope, offset, power)
//! triplets; the SchemaV2 `ColorGrade` carries them as `[r, g, b, master]`
//! arrays so the JSON is compact.

use serde::{Deserialize, Serialize};

/// ASC CDL parameters per channel.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ColorDecisionList {
    /// `[r, g, b, master]` slope (Resolve "gain").
    pub slope: [f32; 4],
    /// `[r, g, b, master]` offset (Resolve "lift").
    pub offset: [f32; 4],
    /// `[r, g, b, master]` power (Resolve "gamma" reciprocal: `1.0 / gamma`).
    pub power: [f32; 4],
    /// Saturation multiplier (1.0 = identity).
    pub saturation: f32,
}

impl Default for ColorDecisionList {
    fn default() -> Self {
        Self {
            slope: [1.0, 1.0, 1.0, 1.0],
            offset: [0.0, 0.0, 0.0, 0.0],
            power: [1.0, 1.0, 1.0, 1.0],
            saturation: 1.0,
        }
    }
}

impl ColorDecisionList {
    /// Construct from the schema-v2 `ColorGrade` shape.
    pub fn from_grade(lift: [f32; 4], gamma: [f32; 4], gain: [f32; 4], saturation: f32) -> Self {
        Self {
            slope: gain,
            offset: lift,
            // Schema gamma is the user-facing gamma; CDL "power" is the inverse.
            power: [
                1.0 / gamma[0].max(1e-3),
                1.0 / gamma[1].max(1e-3),
                1.0 / gamma[2].max(1e-3),
                1.0 / gamma[3].max(1e-3),
            ],
            saturation,
        }
    }
}

/// Apply CDL to a single linear-light RGB pixel in [0, 1]. Reference impl
/// for tests and software preview; the production render path uses the
/// FFmpeg `colorchannelmixer` + `eq` chain in [`super::ffmpeg`].
pub fn apply_cdl_pixel(rgb: [f32; 3], cdl: &ColorDecisionList) -> [f32; 3] {
    let mut out = [0.0; 3];
    for (i, c) in rgb.iter().enumerate() {
        let v = c * cdl.slope[i] * cdl.slope[3] + cdl.offset[i] + cdl.offset[3];
        let v = v.max(0.0).powf(cdl.power[i] * cdl.power[3]);
        out[i] = v;
    }
    // Saturation around BT.709 luma.
    let luma = 0.2126 * out[0] + 0.7152 * out[1] + 0.0722 * out[2];
    for c in out.iter_mut() {
        *c = luma + (*c - luma) * cdl.saturation;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identity_cdl_preserves_input() {
        let cdl = ColorDecisionList::default();
        let rgb = [0.4, 0.5, 0.6];
        let out = apply_cdl_pixel(rgb, &cdl);
        for i in 0..3 {
            assert!((out[i] - rgb[i]).abs() < 1e-5);
        }
    }

    #[test]
    fn saturation_zero_collapses_to_grey() {
        let cdl = ColorDecisionList {
            saturation: 0.0,
            ..ColorDecisionList::default()
        };
        let out = apply_cdl_pixel([1.0, 0.0, 0.0], &cdl);
        let r = out[0];
        // After desaturation all channels equal.
        assert!((out[0] - out[1]).abs() < 1e-5);
        assert!((out[1] - out[2]).abs() < 1e-5);
        // BT.709 luma of pure red is 0.2126.
        assert!((r - 0.2126).abs() < 1e-3);
    }

    #[test]
    fn from_grade_roundtrips_neutral() {
        let cdl = ColorDecisionList::from_grade(
            [0.0, 0.0, 0.0, 0.0],
            [1.0, 1.0, 1.0, 1.0],
            [1.0, 1.0, 1.0, 1.0],
            1.0,
        );
        let out = apply_cdl_pixel([0.3, 0.4, 0.5], &cdl);
        assert!((out[0] - 0.3).abs() < 1e-5);
        assert!((out[1] - 0.4).abs() < 1e-5);
        assert!((out[2] - 0.5).abs() < 1e-5);
    }
}
