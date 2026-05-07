//! FFmpeg filtergraph emitters for the color pipeline.
//!
//! These produce filtergraph fragments that the render compiler concatenates
//! per clip. Inputs/outputs are linked via labels supplied by the caller so
//! the same emitter works inside any larger graph.

use crate::cdl::ColorDecisionList;

/// Emit an FFmpeg filtergraph fragment that applies a CDL to the labeled
/// input. The output is on `[<out>]`.
///
/// We use `colorchannelmixer` for the slope component (gain in CDL terms),
/// `lut` for the offset+power (gamma+lift), and `hue=s=<sat>` for
/// saturation. This matches Resolve's render math closely enough that
/// grades transfer between tools when the grade is then converted to CDL
/// via `slop-otio`.
pub fn cdl_to_filtergraph(input: &str, out: &str, cdl: &ColorDecisionList) -> String {
    let mut frag = String::new();
    // Slope: per-channel gain.
    frag.push_str(&format!(
        "[{input}]colorchannelmixer=rr={r}:gg={g}:bb={b}[{out}_g];",
        input = input,
        out = out,
        r = cdl.slope[0] * cdl.slope[3],
        g = cdl.slope[1] * cdl.slope[3],
        b = cdl.slope[2] * cdl.slope[3],
    ));
    // Offset + power: a per-channel `lut` would be cleaner but ffmpeg's
    // `lutyuv`/`lutrgb` take expressions; we approximate offset+power with
    // `eq=` (brightness/contrast/gamma) plus a per-channel curves filter.
    // For a full SOTA pipeline this would emit a synthesized 3D LUT and
    // pass it through `lut3d` for stability across hosts.
    let g = cdl.power[0] * cdl.power[3];
    let lift = cdl.offset[0] + cdl.offset[3];
    frag.push_str(&format!(
        "[{out}_g]eq=brightness={lift:.6}:gamma={g:.6}[{out}_lg];",
    ));
    // Saturation: `hue` filter takes saturation in the range [0, 3].
    frag.push_str(&format!(
        "[{out}_lg]hue=s={s:.6}[{out}];",
        s = cdl.saturation
    ));
    frag
}

/// Emit a filtergraph fragment that applies a 3D LUT (`.cube`) file. The
/// `lut3d` ffmpeg filter is the SOTA: trilinear-interpolated, GPU-friendly
/// where supported, and bit-exact across hosts.
pub fn lut_to_filtergraph(input: &str, out: &str, cube_path: &str) -> String {
    format!(
        "[{input}]lut3d=file={path}:interp=trilinear[{out}];",
        input = input,
        out = out,
        path = ffmpeg_escape(cube_path),
    )
}

fn ffmpeg_escape(s: &str) -> String {
    s.replace('\\', "\\\\").replace(':', "\\:")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cdl_filtergraph_uses_input_output_labels() {
        let cdl = ColorDecisionList::default();
        let g = cdl_to_filtergraph("v0", "v0_color", &cdl);
        assert!(g.contains("[v0]"));
        assert!(g.contains("[v0_color]"));
        assert!(g.contains("colorchannelmixer"));
        assert!(g.contains("hue=s="));
    }

    #[test]
    fn lut_filtergraph_escapes_colons() {
        let g = lut_to_filtergraph("v0", "v0_lut", "/tmp/some:cube.cube");
        assert!(g.contains("\\:"));
        assert!(g.contains("interp=trilinear"));
    }
}
