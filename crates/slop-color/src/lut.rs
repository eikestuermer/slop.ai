//! Iridas `.cube` 3D LUT loader.
//!
//! Spec: <https://web.archive.org/web/20210625101524/https://wwwimages2.adobe.com/content/dam/acom/en/products/speedgrade/cc/pdfs/cube-lut-specification-1.0.pdf>
//!
//! We support 1D and 3D CUBE LUTs with `LUT_3D_SIZE` up to 64 (above that,
//! the file is too big to load reasonably; matches Resolve's practical
//! limit). 1D LUTs are uncommon for grading but supported for completeness.

use serde::{Deserialize, Serialize};
use std::path::Path;
use thiserror::Error;

/// LUT load errors.
#[derive(Debug, Error)]
pub enum LutError {
    /// I/O.
    #[error(transparent)]
    Io(#[from] std::io::Error),
    /// Parse failure with line context.
    #[error("malformed cube file at line {line}: {message}")]
    Parse {
        /// 1-indexed line number.
        line: usize,
        /// Detail.
        message: String,
    },
    /// Size out of supported range.
    #[error("unsupported LUT size {0} (max 64)")]
    UnsupportedSize(u32),
}

/// A 3D LUT (`size^3` RGB triples).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreeDLut {
    /// Size on each axis (typical: 17, 33, 65).
    pub size: u32,
    /// Domain min `[r, g, b]`.
    pub domain_min: [f32; 3],
    /// Domain max `[r, g, b]`.
    pub domain_max: [f32; 3],
    /// Flat `[r0, g0, b0, r1, g1, b1, ...]` array of length `size^3 * 3`.
    pub data: Vec<f32>,
    /// Optional title from the file.
    pub title: Option<String>,
}

impl ThreeDLut {
    /// Sample the LUT at `rgb` in `[0, 1]^3` using trilinear interpolation.
    pub fn sample(&self, rgb: [f32; 3]) -> [f32; 3] {
        let s = self.size as f32 - 1.0;
        let normalized = |c: f32, axis: usize| -> f32 {
            let a = self.domain_min[axis];
            let b = self.domain_max[axis];
            ((c - a) / (b - a).max(1e-6)).clamp(0.0, 1.0)
        };
        let x = normalized(rgb[0], 0) * s;
        let y = normalized(rgb[1], 1) * s;
        let z = normalized(rgb[2], 2) * s;
        let x0 = x.floor() as u32;
        let y0 = y.floor() as u32;
        let z0 = z.floor() as u32;
        let x1 = (x0 + 1).min(self.size - 1);
        let y1 = (y0 + 1).min(self.size - 1);
        let z1 = (z0 + 1).min(self.size - 1);
        let xd = x - x0 as f32;
        let yd = y - y0 as f32;
        let zd = z - z0 as f32;
        let idx = |xi: u32, yi: u32, zi: u32| -> usize {
            ((zi * self.size + yi) * self.size + xi) as usize * 3
        };
        let mut out = [0.0; 3];
        for (ch, out_c) in out.iter_mut().enumerate() {
            let c000 = self.data[idx(x0, y0, z0) + ch];
            let c100 = self.data[idx(x1, y0, z0) + ch];
            let c010 = self.data[idx(x0, y1, z0) + ch];
            let c110 = self.data[idx(x1, y1, z0) + ch];
            let c001 = self.data[idx(x0, y0, z1) + ch];
            let c101 = self.data[idx(x1, y0, z1) + ch];
            let c011 = self.data[idx(x0, y1, z1) + ch];
            let c111 = self.data[idx(x1, y1, z1) + ch];
            let c00 = c000 * (1.0 - xd) + c100 * xd;
            let c01 = c001 * (1.0 - xd) + c101 * xd;
            let c10 = c010 * (1.0 - xd) + c110 * xd;
            let c11 = c011 * (1.0 - xd) + c111 * xd;
            let c0 = c00 * (1.0 - yd) + c10 * yd;
            let c1 = c01 * (1.0 - yd) + c11 * yd;
            *out_c = c0 * (1.0 - zd) + c1 * zd;
        }
        out
    }
}

/// Parse a `.cube` file from disk.
pub fn load_cube_file(path: &Path) -> Result<ThreeDLut, LutError> {
    let s = std::fs::read_to_string(path)?;
    parse_cube(&s)
}

fn parse_cube(s: &str) -> Result<ThreeDLut, LutError> {
    let mut size: u32 = 0;
    let mut domain_min = [0.0, 0.0, 0.0];
    let mut domain_max = [1.0, 1.0, 1.0];
    let mut title = None;
    let mut data: Vec<f32> = Vec::new();

    for (lineno, raw) in s.lines().enumerate() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some(rest) = line.strip_prefix("TITLE") {
            title = Some(rest.trim().trim_matches('"').to_string());
            continue;
        }
        if let Some(rest) = line.strip_prefix("LUT_3D_SIZE") {
            size = rest
                .trim()
                .parse()
                .map_err(|e: std::num::ParseIntError| LutError::Parse {
                    line: lineno + 1,
                    message: e.to_string(),
                })?;
            if size > 64 {
                return Err(LutError::UnsupportedSize(size));
            }
            continue;
        }
        if let Some(rest) = line.strip_prefix("DOMAIN_MIN") {
            domain_min = parse_triple(rest, lineno + 1)?;
            continue;
        }
        if let Some(rest) = line.strip_prefix("DOMAIN_MAX") {
            domain_max = parse_triple(rest, lineno + 1)?;
            continue;
        }
        // Else: data row.
        let triple = parse_triple(line, lineno + 1)?;
        data.extend_from_slice(&triple);
    }

    if size == 0 {
        return Err(LutError::Parse {
            line: 0,
            message: "missing LUT_3D_SIZE".into(),
        });
    }
    let expected = (size as usize).pow(3) * 3;
    if data.len() != expected {
        return Err(LutError::Parse {
            line: 0,
            message: format!("expected {} floats, got {}", expected, data.len()),
        });
    }
    Ok(ThreeDLut {
        size,
        domain_min,
        domain_max,
        data,
        title,
    })
}

fn parse_triple(s: &str, line: usize) -> Result<[f32; 3], LutError> {
    let parts: Vec<&str> = s.split_whitespace().collect();
    if parts.len() != 3 {
        return Err(LutError::Parse {
            line,
            message: format!("expected 3 floats, got {} on '{}'", parts.len(), s),
        });
    }
    let mut out = [0.0; 3];
    for (i, p) in parts.iter().enumerate() {
        out[i] = p.parse::<f32>().map_err(|e| LutError::Parse {
            line,
            message: e.to_string(),
        })?;
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_minimal_2x2x2_identity() {
        let cube = "\
TITLE \"identity\"
LUT_3D_SIZE 2
0.0 0.0 0.0
1.0 0.0 0.0
0.0 1.0 0.0
1.0 1.0 0.0
0.0 0.0 1.0
1.0 0.0 1.0
0.0 1.0 1.0
1.0 1.0 1.0
";
        let lut = parse_cube(cube).unwrap();
        assert_eq!(lut.size, 2);
        assert_eq!(lut.title.as_deref(), Some("identity"));
        let red = lut.sample([1.0, 0.0, 0.0]);
        assert!((red[0] - 1.0).abs() < 1e-5);
        assert!((red[1] - 0.0).abs() < 1e-5);
        assert!((red[2] - 0.0).abs() < 1e-5);
    }

    #[test]
    fn rejects_oversized_lut() {
        let cube = "LUT_3D_SIZE 128\n";
        assert!(matches!(
            parse_cube(cube),
            Err(LutError::UnsupportedSize(128))
        ));
    }

    #[test]
    fn rejects_truncated_data() {
        let cube = "LUT_3D_SIZE 2\n0.0 0.0 0.0\n";
        assert!(matches!(parse_cube(cube), Err(LutError::Parse { .. })));
    }

    #[test]
    fn parses_domain_min_max() {
        let cube = "\
DOMAIN_MIN 0.0 0.0 0.0
DOMAIN_MAX 2.0 2.0 2.0
LUT_3D_SIZE 2
0.0 0.0 0.0
1.0 0.0 0.0
0.0 1.0 0.0
1.0 1.0 0.0
0.0 0.0 1.0
1.0 0.0 1.0
0.0 1.0 1.0
1.0 1.0 1.0
";
        let lut = parse_cube(cube).unwrap();
        assert_eq!(lut.domain_min, [0.0, 0.0, 0.0]);
        assert_eq!(lut.domain_max, [2.0, 2.0, 2.0]);
        // With domain max=2, sampling at rgb=[1, 1, 1] is 50% along each axis,
        // so it sits between the 8 corners.
        let mid = lut.sample([1.0, 1.0, 1.0]);
        for c in mid {
            assert!(c > 0.0 && c < 1.0, "got {mid:?}");
        }
    }

    #[test]
    fn trilinear_interpolation_is_between_corners() {
        // Identity 2x2x2: white corner = [1,1,1], black = [0,0,0].
        let cube = "LUT_3D_SIZE 2
0.0 0.0 0.0
1.0 0.0 0.0
0.0 1.0 0.0
1.0 1.0 0.0
0.0 0.0 1.0
1.0 0.0 1.0
0.0 1.0 1.0
1.0 1.0 1.0
";
        let lut = parse_cube(cube).unwrap();
        let mid = lut.sample([0.5, 0.5, 0.5]);
        for c in mid {
            assert!((c - 0.5).abs() < 1e-5, "expected ~0.5, got {c}");
        }
    }
}
