//! FFmpeg-backed scope generators (waveform, vectorscope, parade, histogram).
//!
//! These emit filtergraph fragments that produce video streams suitable for
//! display in the inspector panel. The render compiler hooks them into a
//! tap that doesn't affect the program output.

use serde::{Deserialize, Serialize};

/// Which scope to render.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ScopeKind {
    /// Luma waveform.
    Waveform,
    /// RGB parade.
    Parade,
    /// Vectorscope.
    Vectorscope,
    /// Histogram.
    Histogram,
}

/// Knobs.
#[derive(Debug, Clone)]
pub struct ScopeOptions {
    /// Output width.
    pub width: u32,
    /// Output height.
    pub height: u32,
}

impl Default for ScopeOptions {
    fn default() -> Self {
        Self {
            width: 256,
            height: 256,
        }
    }
}

/// Emit a filtergraph fragment that generates the requested scope from
/// `[input]` into `[output]`.
pub fn scope_filtergraph(
    input: &str,
    output: &str,
    kind: ScopeKind,
    opts: &ScopeOptions,
) -> String {
    match kind {
        ScopeKind::Waveform => format!(
            "[{input}]split[{input}_wfsrc][{output}_pass];[{input}_wfsrc]waveform=mode=column:components=1:display=stack,scale={w}:{h}[{output}];",
            input = input,
            output = output,
            w = opts.width,
            h = opts.height,
        ),
        ScopeKind::Parade => format!(
            "[{input}]waveform=mode=column:components=7:display=parade,scale={w}:{h}[{output}];",
            input = input,
            output = output,
            w = opts.width,
            h = opts.height,
        ),
        ScopeKind::Vectorscope => format!(
            "[{input}]vectorscope=mode=color3:graticule=green:flags=name,scale={w}:{h}[{output}];",
            input = input,
            output = output,
            w = opts.width,
            h = opts.height,
        ),
        ScopeKind::Histogram => format!(
            "[{input}]histogram=display_mode=stack:components=7,scale={w}:{h}[{output}];",
            input = input,
            output = output,
            w = opts.width,
            h = opts.height,
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_scope_kind_emits_input_and_output_labels() {
        for kind in [
            ScopeKind::Waveform,
            ScopeKind::Parade,
            ScopeKind::Vectorscope,
            ScopeKind::Histogram,
        ] {
            let g = scope_filtergraph("v0", "scope", kind, &ScopeOptions::default());
            assert!(g.contains("[v0]"), "kind={kind:?}: {g}");
            assert!(g.contains("[scope]"), "kind={kind:?}: {g}");
        }
    }
}
