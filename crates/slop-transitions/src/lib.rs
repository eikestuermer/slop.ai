//! # slop-transitions
//!
//! FFmpeg-backed transition library. Each entry maps a V2 schema
//! `TransitionItem.kind` to an `xfade` transition name plus an optional
//! audio crossfade.
//!
//! `xfade` is the de-facto SOTA for software transitions in modern FFmpeg
//! (≥ 4.3): GPU-friendly via the `vulkan_xfade` companion, deterministic,
//! covers ~50 named transitions out of the box. Slop AI exposes a curated
//! 15 to keep the planner-side palette manageable.

#![deny(missing_docs)]

use serde::Serialize;

/// One declarative transition definition.
#[derive(Debug, Clone, Serialize)]
pub struct Transition {
    /// V2 schema kind string.
    pub kind: &'static str,
    /// `xfade` `transition=` value.
    pub xfade: &'static str,
    /// Whether to also apply audio cross-fade (always true for our V1.5
    /// catalog).
    pub audio_crossfade: bool,
    /// Default duration in seconds.
    pub default_duration_sec: f64,
}

/// V1.5 transition catalog. Order is stable: schema upgrades append, never
/// reorder.
pub const CATALOG: &[Transition] = &[
    Transition {
        kind: "cross_dissolve",
        xfade: "fade",
        audio_crossfade: true,
        default_duration_sec: 0.5,
    },
    Transition {
        kind: "fade_to_black",
        xfade: "fadeblack",
        audio_crossfade: true,
        default_duration_sec: 0.7,
    },
    Transition {
        kind: "fade_to_white",
        xfade: "fadewhite",
        audio_crossfade: true,
        default_duration_sec: 0.7,
    },
    Transition {
        kind: "wipe_left",
        xfade: "wipeleft",
        audio_crossfade: true,
        default_duration_sec: 0.4,
    },
    Transition {
        kind: "wipe_right",
        xfade: "wiperight",
        audio_crossfade: true,
        default_duration_sec: 0.4,
    },
    Transition {
        kind: "wipe_up",
        xfade: "wipeup",
        audio_crossfade: true,
        default_duration_sec: 0.4,
    },
    Transition {
        kind: "wipe_down",
        xfade: "wipedown",
        audio_crossfade: true,
        default_duration_sec: 0.4,
    },
    Transition {
        kind: "iris_open",
        xfade: "circleopen",
        audio_crossfade: true,
        default_duration_sec: 0.5,
    },
    Transition {
        kind: "iris_close",
        xfade: "circleclose",
        audio_crossfade: true,
        default_duration_sec: 0.5,
    },
    Transition {
        kind: "push_left",
        xfade: "slideleft",
        audio_crossfade: true,
        default_duration_sec: 0.4,
    },
    Transition {
        kind: "push_right",
        xfade: "slideright",
        audio_crossfade: true,
        default_duration_sec: 0.4,
    },
    Transition {
        kind: "slide_up",
        xfade: "slideup",
        audio_crossfade: true,
        default_duration_sec: 0.4,
    },
    Transition {
        kind: "slide_down",
        xfade: "slidedown",
        audio_crossfade: true,
        default_duration_sec: 0.4,
    },
    Transition {
        kind: "zoom_blur",
        xfade: "zoomin",
        audio_crossfade: true,
        default_duration_sec: 0.5,
    },
    Transition {
        kind: "luma_dissolve",
        xfade: "smoothleft",
        audio_crossfade: true,
        default_duration_sec: 0.5,
    },
];

/// Look up a transition by V2 schema kind.
pub fn find(kind: &str) -> Option<&'static Transition> {
    CATALOG.iter().find(|t| t.kind == kind)
}

/// Emit an `xfade` filtergraph fragment for a video transition.
///
/// The two inputs `[a]` and `[b]` are concatenated with the transition
/// applied. `start_time_sec` is the time on the `[a]` stream where the
/// transition begins.
pub fn video_xfade(
    a: &str,
    b: &str,
    out: &str,
    kind: &str,
    duration_sec: f64,
    start_time_sec: f64,
) -> Option<String> {
    let t = find(kind)?;
    Some(format!(
        "[{a}][{b}]xfade=transition={x}:duration={d}:offset={o}[{out}];",
        a = a,
        b = b,
        out = out,
        x = t.xfade,
        d = duration_sec,
        o = start_time_sec,
    ))
}

/// Emit an `acrossfade` filtergraph fragment.
pub fn audio_crossfade(a: &str, b: &str, out: &str, duration_sec: f64) -> String {
    format!(
        "[{a}][{b}]acrossfade=d={d}:c1=tri:c2=tri[{out}];",
        a = a,
        b = b,
        out = out,
        d = duration_sec,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_has_15_entries() {
        assert_eq!(CATALOG.len(), 15);
    }

    #[test]
    fn find_returns_known_kind() {
        let t = find("cross_dissolve").unwrap();
        assert_eq!(t.xfade, "fade");
    }

    #[test]
    fn xfade_emits_correct_filter() {
        let g = video_xfade("v0", "v1", "vt", "wipe_left", 0.5, 4.5).unwrap();
        assert!(g.contains("transition=wipeleft"));
        assert!(g.contains("offset=4.5"));
        assert!(g.contains("duration=0.5"));
    }

    #[test]
    fn unknown_kind_is_none() {
        assert!(video_xfade("v0", "v1", "vt", "zoom_blur_supreme", 0.5, 0.0).is_none());
    }

    #[test]
    fn every_catalog_entry_has_non_empty_xfade_mapping() {
        for t in CATALOG {
            assert!(!t.xfade.is_empty(), "kind {} has empty xfade", t.kind);
            assert!(!t.kind.is_empty());
            assert!(t.default_duration_sec > 0.0);
        }
    }

    #[test]
    fn audio_crossfade_uses_triangular_curves() {
        let g = audio_crossfade("a0", "a1", "ax", 0.4);
        assert!(g.contains("c1=tri"));
        assert!(g.contains("c2=tri"));
        assert!(g.contains("d=0.4"));
    }
}
