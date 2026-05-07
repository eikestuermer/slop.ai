//! Deterministic repair pass for `Plan`s that fail [`crate::validator`].
//!
//! This is intentionally narrow: we never silently change the model's
//! creative intent. We only fix mechanical errors that the validator already
//! flagged:
//!
//! - clamp `src_out` to the asset duration,
//! - clamp `src_in` to >= 0,
//! - drop clips whose `asset_id` is not in the candidate set,
//! - sort clips on each track by `timeline_in` and remove backward overlap,
//! - drop clips with `src_in >= src_out` after clamping.
//!
//! Anything left after repair that still fails the validator is surfaced to
//! the caller, which should send a single targeted "fix these issues"
//! message back to the model rather than guessing further.

use crate::{plan::*, timeline::Timeline};

/// Apply mechanical repairs in place. Returns the list of human-readable
/// notes describing every change.
pub fn repair_plan(plan: &mut Plan, tl: &Timeline) -> Vec<String> {
    let mut notes = Vec::new();
    let mut known_assets = std::collections::HashSet::new();
    for a in &tl.assets {
        known_assets.insert(a.asset_id.clone());
    }

    for track in &mut plan.timeline.tracks {
        // Drop clips that reference unknown assets.
        track.clips.retain(|c| {
            if known_assets.contains(&c.asset_id) {
                true
            } else {
                notes.push(format!("dropped clip with unknown asset_id {}", c.asset_id));
                false
            }
        });

        // Clamp src ranges to known asset durations.
        for clip in &mut track.clips {
            if let Some(asset) = tl.asset(&clip.asset_id) {
                if clip.src_in < 0.0 {
                    notes.push(format!(
                        "clamped src_in {} -> 0 for {}",
                        clip.src_in, clip.asset_id
                    ));
                    clip.src_in = 0.0;
                }
                if clip.src_out > asset.duration_sec {
                    notes.push(format!(
                        "clamped src_out {} -> {} for {}",
                        clip.src_out, asset.duration_sec, clip.asset_id
                    ));
                    clip.src_out = asset.duration_sec;
                }
            }
        }

        // Drop clips with collapsed source ranges.
        let before = track.clips.len();
        track.clips.retain(|c| c.src_out > c.src_in + 1e-3);
        if track.clips.len() != before {
            notes.push(format!(
                "dropped {} clip(s) with empty source range on track {}",
                before - track.clips.len(),
                track.id
            ));
        }

        // Sort by timeline_in.
        track
            .clips
            .sort_by(|a, b| a.timeline_in.partial_cmp(&b.timeline_in).unwrap());

        // Eliminate overlap by re-flowing timeline_in to the previous clip's
        // end. We never *extend* a clip; we only push it later.
        let mut last_end = 0.0_f64;
        for clip in &mut track.clips {
            let dur = clip.src_out - clip.src_in;
            if clip.timeline_in + 1e-6 < last_end {
                notes.push(format!(
                    "shifted clip ({}@{:.3}) to {:.3} to fix overlap on track {}",
                    clip.asset_id, clip.timeline_in, last_end, track.id
                ));
                clip.timeline_in = last_end;
            }
            last_end = clip.timeline_in + dur;
        }
    }

    notes
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids;
    use crate::timeline::*;

    fn fixture_tl() -> Timeline {
        let mut tl = Timeline::empty();
        tl.assets.push(Asset {
            asset_id: "a1".into(),
            uri: "file:///a.mp4".into(),
            duration_sec: 30.0,
            has_video: true,
            has_audio: true,
            fps: None,
            resolution: None,
            transcript_ref: None,
            shot_list_ref: None,
        });
        let _ = ids::track();
        tl
    }

    #[test]
    fn drops_unknown_assets() {
        let tl = fixture_tl();
        let mut plan = Plan {
            version: "roughcut_plan.v1".into(),
            summary: "x".into(),
            timeline: PlanTimeline {
                fps: 30.0,
                tracks: vec![PlanTrack {
                    kind: "video".into(),
                    id: "v1".into(),
                    clips: vec![
                        PlannedClip {
                            asset_id: "a1".into(),
                            segment_id: None,
                            shot_id: None,
                            src_in: 0.0,
                            src_out: 5.0,
                            timeline_in: 0.0,
                            lane: 0,
                            reason: "ok".into(),
                        },
                        PlannedClip {
                            asset_id: "a_does_not_exist".into(),
                            segment_id: None,
                            shot_id: None,
                            src_in: 0.0,
                            src_out: 5.0,
                            timeline_in: 5.0,
                            lane: 0,
                            reason: "bogus".into(),
                        },
                    ],
                }],
            },
            captions: vec![],
            warnings: vec![],
        };
        let notes = repair_plan(&mut plan, &tl);
        assert_eq!(plan.timeline.tracks[0].clips.len(), 1);
        assert!(notes.iter().any(|n| n.contains("unknown asset_id")));
    }

    #[test]
    fn clamps_src_out_and_fixes_overlap() {
        let tl = fixture_tl();
        let mut plan = Plan {
            version: "roughcut_plan.v1".into(),
            summary: "x".into(),
            timeline: PlanTimeline {
                fps: 30.0,
                tracks: vec![PlanTrack {
                    kind: "video".into(),
                    id: "v1".into(),
                    clips: vec![
                        PlannedClip {
                            asset_id: "a1".into(),
                            segment_id: None,
                            shot_id: None,
                            src_in: 0.0,
                            src_out: 100.0, // out of range
                            timeline_in: 0.0,
                            lane: 0,
                            reason: "first".into(),
                        },
                        PlannedClip {
                            asset_id: "a1".into(),
                            segment_id: None,
                            shot_id: None,
                            src_in: 5.0,
                            src_out: 10.0,
                            timeline_in: 1.0, // overlaps the first
                            lane: 0,
                            reason: "second".into(),
                        },
                    ],
                }],
            },
            captions: vec![],
            warnings: vec![],
        };
        let notes = repair_plan(&mut plan, &tl);
        let track = &plan.timeline.tracks[0];
        assert_eq!(track.clips[0].src_out, 30.0);
        // After clamping, first clip ends at 30s; second must be shifted.
        assert!(track.clips[1].timeline_in >= 30.0 - 1e-6);
        assert!(notes.iter().any(|n| n.contains("clamped src_out")));
        assert!(notes.iter().any(|n| n.contains("shifted clip")));
    }

    #[test]
    fn clamps_negative_src_in() {
        let tl = fixture_tl();
        let mut plan = Plan {
            version: "roughcut_plan.v1".into(),
            summary: "x".into(),
            timeline: PlanTimeline {
                fps: 30.0,
                tracks: vec![PlanTrack {
                    kind: "video".into(),
                    id: "v1".into(),
                    clips: vec![PlannedClip {
                        asset_id: "a1".into(),
                        segment_id: None,
                        shot_id: None,
                        src_in: -2.5,
                        src_out: 5.0,
                        timeline_in: 0.0,
                        lane: 0,
                        reason: "starts before zero".into(),
                    }],
                }],
            },
            captions: vec![],
            warnings: vec![],
        };
        let notes = repair_plan(&mut plan, &tl);
        assert_eq!(plan.timeline.tracks[0].clips[0].src_in, 0.0);
        assert!(notes.iter().any(|n| n.contains("clamped src_in")));
    }

    #[test]
    fn dropped_invalid_asset_preserves_remaining_clips_on_multi_track_plans() {
        let tl = fixture_tl();
        let mut plan = Plan {
            version: "roughcut_plan.v1".into(),
            summary: "x".into(),
            timeline: PlanTimeline {
                fps: 30.0,
                tracks: vec![
                    PlanTrack {
                        kind: "video".into(),
                        id: "v1".into(),
                        clips: vec![
                            PlannedClip {
                                asset_id: "a1".into(),
                                segment_id: None,
                                shot_id: None,
                                src_in: 0.0,
                                src_out: 5.0,
                                timeline_in: 0.0,
                                lane: 0,
                                reason: "valid 1".into(),
                            },
                            PlannedClip {
                                asset_id: "ghost".into(),
                                segment_id: None,
                                shot_id: None,
                                src_in: 0.0,
                                src_out: 5.0,
                                timeline_in: 5.0,
                                lane: 0,
                                reason: "ghost".into(),
                            },
                        ],
                    },
                    PlanTrack {
                        kind: "audio".into(),
                        id: "a1t".into(),
                        clips: vec![PlannedClip {
                            asset_id: "a1".into(),
                            segment_id: None,
                            shot_id: None,
                            src_in: 0.0,
                            src_out: 5.0,
                            timeline_in: 0.0,
                            lane: 0,
                            reason: "valid audio".into(),
                        }],
                    },
                ],
            },
            captions: vec![],
            warnings: vec![],
        };
        repair_plan(&mut plan, &tl);
        // Track v1: ghost dropped, valid clip kept.
        assert_eq!(plan.timeline.tracks[0].clips.len(), 1);
        assert_eq!(plan.timeline.tracks[0].clips[0].asset_id, "a1");
        // Track a1t: untouched.
        assert_eq!(plan.timeline.tracks[1].clips.len(), 1);
    }
}
