//! The reducer: apply an `Op` to a `Timeline`.
//!
//! The reducer is the *only* path by which a `Timeline` is mutated. It
//! enforces invariants by returning an `Error` rather than mutating into an
//! invalid state. Callers that want to be safe under concurrent edits should
//! always go: validate -> apply -> persist op to `ops.jsonl`.

use crate::{error::Result, ops::*, timeline::*, Error};

/// Apply `op` to `tl`, mutating it in place. Returns the inverse op (suitable
/// for undo). On error, `tl` is left unchanged for the operations that fail
/// before any mutation; structural mutations (e.g. `ReplaceTimelineRange`)
/// are best-effort and may leave the timeline in a partially updated state.
pub fn apply(tl: &mut Timeline, op: &Op) -> Result<Op> {
    let inverse_kind = match &op.kind {
        OpKind::AddAsset(asset) => {
            if tl.assets.iter().any(|a| a.asset_id == asset.asset_id) {
                return Err(Error::Invariant(format!(
                    "duplicate asset_id {}",
                    asset.asset_id
                )));
            }
            tl.assets.push(asset.clone());
            OpKind::RemoveAsset {
                asset_id: asset.asset_id.clone(),
            }
        }

        OpKind::RemoveAsset { asset_id } => {
            let pos = tl
                .assets
                .iter()
                .position(|a| &a.asset_id == asset_id)
                .ok_or_else(|| Error::UnknownAsset(asset_id.clone()))?;
            let removed = tl.assets.remove(pos);
            OpKind::AddAsset(removed)
        }

        OpKind::AddTrack { track_id, kind } => {
            if tl.tracks.iter().any(|t| &t.track_id == track_id) {
                return Err(Error::Invariant(format!("duplicate track_id {track_id}")));
            }
            tl.tracks.push(Track {
                track_id: track_id.clone(),
                kind: *kind,
                items: Vec::new(),
            });
            OpKind::RemoveTrack {
                track_id: track_id.clone(),
            }
        }

        OpKind::RemoveTrack { track_id } => {
            let pos = tl
                .tracks
                .iter()
                .position(|t| &t.track_id == track_id)
                .ok_or_else(|| Error::UnknownTrack(track_id.clone()))?;
            let track = tl.tracks.remove(pos);
            OpKind::ReinsertTrack { track, index: pos }
        }

        OpKind::ReinsertTrack { track, index } => {
            let idx = (*index).min(tl.tracks.len());
            tl.tracks.insert(idx, track.clone());
            OpKind::RemoveTrack {
                track_id: track.track_id.clone(),
            }
        }

        OpKind::InsertClip { track_id, clip } => {
            if !tl.assets.iter().any(|a| a.asset_id == clip.asset_id) {
                return Err(Error::UnknownAsset(clip.asset_id.clone()));
            }
            let track = tl
                .track_mut(track_id)
                .ok_or_else(|| Error::UnknownTrack(track_id.clone()))?;
            if track.items.iter().any(|i| i.id() == clip.item_id) {
                return Err(Error::Invariant(format!(
                    "duplicate item_id {}",
                    clip.item_id
                )));
            }
            track.items.push(TrackItem::Clip(clip.clone()));
            sort_track(track);
            OpKind::RemoveClip {
                track_id: track_id.clone(),
                item_id: clip.item_id.clone(),
            }
        }

        OpKind::RemoveClip { track_id, item_id } => {
            let track = tl
                .track_mut(track_id)
                .ok_or_else(|| Error::UnknownTrack(track_id.clone()))?;
            let pos = track
                .items
                .iter()
                .position(|i| i.id() == item_id)
                .ok_or_else(|| Error::Invariant(format!("unknown item {item_id}")))?;
            let removed = track.items.remove(pos);
            match removed {
                TrackItem::Clip(c) => OpKind::InsertClip {
                    track_id: track_id.clone(),
                    clip: c,
                },
                TrackItem::Gap(g) => OpKind::ReinsertGap {
                    track_id: track_id.clone(),
                    gap: g,
                },
            }
        }

        OpKind::ReinsertGap { track_id, gap } => {
            let track = tl
                .track_mut(track_id)
                .ok_or_else(|| Error::UnknownTrack(track_id.clone()))?;
            track.items.push(TrackItem::Gap(gap.clone()));
            sort_track(track);
            OpKind::RemoveClip {
                track_id: track_id.clone(),
                item_id: gap.item_id.clone(),
            }
        }

        OpKind::TrimClip {
            track_id,
            item_id,
            new_src_in,
            new_src_out,
        } => {
            if *new_src_in >= *new_src_out {
                return Err(Error::SrcEmpty(item_id.clone()));
            }
            let track = tl
                .track_mut(track_id)
                .ok_or_else(|| Error::UnknownTrack(track_id.clone()))?;
            let item = track
                .items
                .iter_mut()
                .find(|i| i.id() == item_id)
                .ok_or_else(|| Error::Invariant(format!("unknown item {item_id}")))?;
            let TrackItem::Clip(c) = item else {
                return Err(Error::Invariant(format!("not a clip: {item_id}")));
            };
            let old_in = c.src_in;
            let old_out = c.src_out;
            c.src_in = *new_src_in;
            c.src_out = *new_src_out;
            // Adjust timeline_out so the visible duration reflects the new
            // source range at the current speed.
            let dur = (c.src_out - c.src_in) / c.speed;
            c.timeline_out = c.timeline_in + dur;
            OpKind::TrimClip {
                track_id: track_id.clone(),
                item_id: item_id.clone(),
                new_src_in: old_in,
                new_src_out: old_out,
            }
        }

        OpKind::MoveClip {
            track_id,
            item_id,
            new_timeline_in,
        } => {
            let track = tl
                .track_mut(track_id)
                .ok_or_else(|| Error::UnknownTrack(track_id.clone()))?;
            let item = track
                .items
                .iter_mut()
                .find(|i| i.id() == item_id)
                .ok_or_else(|| Error::Invariant(format!("unknown item {item_id}")))?;
            let TrackItem::Clip(c) = item else {
                return Err(Error::Invariant(format!("not a clip: {item_id}")));
            };
            let old_in = c.timeline_in;
            let dur = c.timeline_out - c.timeline_in;
            c.timeline_in = *new_timeline_in;
            c.timeline_out = *new_timeline_in + dur;
            sort_track(track);
            OpKind::MoveClip {
                track_id: track_id.clone(),
                item_id: item_id.clone(),
                new_timeline_in: old_in,
            }
        }

        OpKind::PinClip { track_id, item_id } => {
            set_lock(tl, track_id, item_id, true)?;
            OpKind::UnpinClip {
                track_id: track_id.clone(),
                item_id: item_id.clone(),
            }
        }
        OpKind::UnpinClip { track_id, item_id } => {
            set_lock(tl, track_id, item_id, false)?;
            OpKind::PinClip {
                track_id: track_id.clone(),
                item_id: item_id.clone(),
            }
        }

        OpKind::AddMarker {
            track_id,
            item_id,
            marker,
        } => {
            let track = tl
                .track_mut(track_id)
                .ok_or_else(|| Error::UnknownTrack(track_id.clone()))?;
            let item = track
                .items
                .iter_mut()
                .find(|i| i.id() == item_id)
                .ok_or_else(|| Error::Invariant(format!("unknown item {item_id}")))?;
            let TrackItem::Clip(c) = item else {
                return Err(Error::Invariant(format!("not a clip: {item_id}")));
            };
            c.markers.push(marker.clone());
            OpKind::RemoveMarker {
                track_id: track_id.clone(),
                item_id: item_id.clone(),
                marker_index: c.markers.len() - 1,
            }
        }

        OpKind::RemoveMarker {
            track_id,
            item_id,
            marker_index,
        } => {
            let track = tl
                .track_mut(track_id)
                .ok_or_else(|| Error::UnknownTrack(track_id.clone()))?;
            let item = track
                .items
                .iter_mut()
                .find(|i| i.id() == item_id)
                .ok_or_else(|| Error::Invariant(format!("unknown item {item_id}")))?;
            let TrackItem::Clip(c) = item else {
                return Err(Error::Invariant(format!("not a clip: {item_id}")));
            };
            if *marker_index >= c.markers.len() {
                return Err(Error::Invariant(format!(
                    "marker index {marker_index} out of range"
                )));
            }
            let removed = c.markers.remove(*marker_index);
            OpKind::AddMarker {
                track_id: track_id.clone(),
                item_id: item_id.clone(),
                marker: removed,
            }
        }

        OpKind::AddCaption(cap) => {
            tl.captions.push(cap.clone());
            OpKind::RemoveCaption {
                index: tl.captions.len() - 1,
            }
        }

        OpKind::RemoveCaption { index } => {
            if *index >= tl.captions.len() {
                return Err(Error::Invariant(format!(
                    "caption index {index} out of range"
                )));
            }
            let removed = tl.captions.remove(*index);
            OpKind::AddCaption(removed)
        }

        OpKind::ReplaceTimelineRange {
            track_id,
            timeline_in,
            timeline_out,
            new_items,
            new_captions,
        } => {
            // Capture the inverse first (the items and captions we are about
            // to drop) so that undo replays cleanly.
            let track = tl
                .track_mut(track_id)
                .ok_or_else(|| Error::UnknownTrack(track_id.clone()))?;
            let mut removed_items = Vec::new();
            let mut keep = Vec::new();
            for item in track.items.drain(..) {
                let in_range =
                    item.timeline_in() >= *timeline_in && item.timeline_out() <= *timeline_out;
                let pinned = match &item {
                    TrackItem::Clip(c) => c.metadata.locked_by_user,
                    TrackItem::Gap(_) => false,
                };
                if in_range && !pinned {
                    removed_items.push(item);
                } else {
                    keep.push(item);
                }
            }
            track.items = keep;
            for c in new_items {
                track.items.push(TrackItem::Clip(c.clone()));
            }
            sort_track(track);

            let removed_captions: Vec<Caption> = tl
                .captions
                .iter()
                .filter(|c| c.timeline_in >= *timeline_in && c.timeline_out <= *timeline_out)
                .cloned()
                .collect();
            tl.captions
                .retain(|c| !(c.timeline_in >= *timeline_in && c.timeline_out <= *timeline_out));
            for c in new_captions {
                tl.captions.push(c.clone());
            }

            OpKind::RestoreTimelineRange {
                track_id: track_id.clone(),
                timeline_in: *timeline_in,
                timeline_out: *timeline_out,
                items: removed_items,
                captions: removed_captions,
            }
        }

        OpKind::RestoreTimelineRange {
            track_id,
            timeline_in,
            timeline_out,
            items,
            captions,
        } => {
            let track = tl
                .track_mut(track_id)
                .ok_or_else(|| Error::UnknownTrack(track_id.clone()))?;
            // Drop everything currently in the range that is not pinned, then
            // splice the captured snapshot back.
            track.items.retain(|i| {
                if i.timeline_in() >= *timeline_in && i.timeline_out() <= *timeline_out {
                    matches!(i, TrackItem::Clip(c) if c.metadata.locked_by_user)
                } else {
                    true
                }
            });
            for i in items {
                track.items.push(i.clone());
            }
            sort_track(track);

            tl.captions
                .retain(|c| !(c.timeline_in >= *timeline_in && c.timeline_out <= *timeline_out));
            for c in captions {
                tl.captions.push(c.clone());
            }

            // The inverse of restore is replace-with-empty, but we don't need
            // to chain further inverses for V1.
            OpKind::ReplaceTimelineRange {
                track_id: track_id.clone(),
                timeline_in: *timeline_in,
                timeline_out: *timeline_out,
                new_items: vec![],
                new_captions: vec![],
            }
        }

        OpKind::SetProjectSettings(p) => {
            let old = tl.project.clone();
            tl.project = p.clone();
            OpKind::SetProjectSettings(old)
        }
    };

    Ok(Op {
        op_id: format!("inv_{}", &op.op_id),
        ts: op.ts,
        actor: op.actor,
        prompt_id: op.prompt_id.clone(),
        kind: inverse_kind,
    })
}

/// Replay an entire op log against an empty timeline. Used for crash
/// recovery and testing.
pub fn replay(ops: &[Op]) -> Result<Timeline> {
    let mut tl = Timeline::empty();
    for op in ops {
        apply(&mut tl, op)?;
    }
    Ok(tl)
}

fn sort_track(track: &mut Track) {
    track
        .items
        .sort_by(|a, b| a.timeline_in().partial_cmp(&b.timeline_in()).unwrap());
}

fn set_lock(tl: &mut Timeline, track_id: &str, item_id: &str, lock: bool) -> Result<()> {
    let track = tl
        .track_mut(track_id)
        .ok_or_else(|| Error::UnknownTrack(track_id.to_string()))?;
    let item = track
        .items
        .iter_mut()
        .find(|i| i.id() == item_id)
        .ok_or_else(|| Error::Invariant(format!("unknown item {item_id}")))?;
    let TrackItem::Clip(c) = item else {
        return Err(Error::Invariant(format!("not a clip: {item_id}")));
    };
    c.metadata.locked_by_user = lock;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids;

    fn fixture_asset(dur: f64) -> Asset {
        Asset {
            asset_id: ids::asset(),
            uri: "file:///tmp/x.mp4".into(),
            duration_sec: dur,
            has_video: true,
            has_audio: true,
            fps: Some(30.0),
            resolution: Some(Resolution { w: 1920, h: 1080 }),
            transcript_ref: None,
            shot_list_ref: None,
        }
    }

    fn fixture_clip(asset_id: &str, src_in: f64, src_out: f64, t_in: f64) -> ClipItem {
        ClipItem {
            item_id: ids::clip(),
            asset_id: asset_id.into(),
            src_in,
            src_out,
            timeline_in: t_in,
            timeline_out: t_in + (src_out - src_in),
            speed: 1.0,
            effects: vec![],
            markers: vec![],
            metadata: ClipMetadata::default(),
        }
    }

    #[test]
    fn add_then_remove_asset_round_trips() {
        let mut tl = Timeline::empty();
        let asset = fixture_asset(60.0);
        let id = asset.asset_id.clone();
        let inv = apply(&mut tl, &Op::new(OpKind::AddAsset(asset.clone()))).unwrap();
        assert_eq!(tl.assets.len(), 1);
        apply(&mut tl, &Op::new(inv.kind.clone())).unwrap();
        assert_eq!(tl.assets.len(), 0);
        assert!(matches!(inv.kind, OpKind::RemoveAsset { asset_id } if asset_id == id));
    }

    #[test]
    fn insert_clip_requires_known_asset_and_track() {
        let mut tl = Timeline::empty();
        let asset = fixture_asset(60.0);
        let aid = asset.asset_id.clone();
        apply(&mut tl, &Op::new(OpKind::AddAsset(asset))).unwrap();
        // No track yet.
        let clip = fixture_clip(&aid, 0.0, 5.0, 0.0);
        let r = apply(
            &mut tl,
            &Op::new(OpKind::InsertClip {
                track_id: "t_does_not_exist".into(),
                clip: clip.clone(),
            }),
        );
        assert!(matches!(r, Err(Error::UnknownTrack(_))));

        // Add a track, now it should succeed.
        let tid = ids::track();
        apply(
            &mut tl,
            &Op::new(OpKind::AddTrack {
                track_id: tid.clone(),
                kind: TrackKind::Video,
            }),
        )
        .unwrap();
        apply(
            &mut tl,
            &Op::new(OpKind::InsertClip {
                track_id: tid.clone(),
                clip,
            }),
        )
        .unwrap();
        assert_eq!(tl.tracks[0].items.len(), 1);
    }

    #[test]
    fn replay_rebuilds_state() {
        let mut tl = Timeline::empty();
        let asset = fixture_asset(60.0);
        let aid = asset.asset_id.clone();
        let tid = ids::track();
        let mut log = OpLog::new();

        for op in [
            Op::new(OpKind::AddAsset(asset)),
            Op::new(OpKind::AddTrack {
                track_id: tid.clone(),
                kind: TrackKind::Video,
            }),
            Op::new(OpKind::InsertClip {
                track_id: tid.clone(),
                clip: fixture_clip(&aid, 0.0, 5.0, 0.0),
            }),
            Op::new(OpKind::InsertClip {
                track_id: tid.clone(),
                clip: fixture_clip(&aid, 10.0, 15.0, 5.0),
            }),
        ] {
            apply(&mut tl, &op).unwrap();
            log.push(op);
        }

        let replayed = replay(log.ops()).unwrap();
        assert_eq!(replayed, tl);
    }

    #[test]
    fn replace_range_respects_pins() {
        let mut tl = Timeline::empty();
        let asset = fixture_asset(60.0);
        let aid = asset.asset_id.clone();
        let tid = ids::track();
        apply(&mut tl, &Op::new(OpKind::AddAsset(asset))).unwrap();
        apply(
            &mut tl,
            &Op::new(OpKind::AddTrack {
                track_id: tid.clone(),
                kind: TrackKind::Video,
            }),
        )
        .unwrap();
        let mut clip_a = fixture_clip(&aid, 0.0, 5.0, 0.0);
        clip_a.metadata.locked_by_user = true;
        let pinned_id = clip_a.item_id.clone();
        let clip_b = fixture_clip(&aid, 10.0, 15.0, 5.0);

        apply(
            &mut tl,
            &Op::new(OpKind::InsertClip {
                track_id: tid.clone(),
                clip: clip_a,
            }),
        )
        .unwrap();
        apply(
            &mut tl,
            &Op::new(OpKind::InsertClip {
                track_id: tid.clone(),
                clip: clip_b,
            }),
        )
        .unwrap();

        let replacement = fixture_clip(&aid, 20.0, 23.0, 5.0);
        apply(
            &mut tl,
            &Op::new(OpKind::ReplaceTimelineRange {
                track_id: tid.clone(),
                timeline_in: 0.0,
                timeline_out: 10.0,
                new_items: vec![replacement.clone()],
                new_captions: vec![],
            }),
        )
        .unwrap();

        let track = tl.track(&tid).unwrap();
        // Pinned clip survived.
        assert!(track.items.iter().any(|i| i.id() == pinned_id));
        // Replacement clip is present.
        assert!(track.items.iter().any(|i| i.id() == replacement.item_id));
        // Unpinned clip B was removed.
        assert_eq!(track.items.len(), 2);
    }
}
