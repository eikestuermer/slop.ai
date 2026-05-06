//! End-to-end integration test: build a project via `Op`s, persist the log,
//! reload from disk, and verify the rebuilt timeline matches.
//!
//! This is the V1 crash-recovery promise.

use slop_core::*;

#[test]
fn ops_log_round_trips_through_disk() {
    let dir = std::env::temp_dir().join(format!(
        "slop-core-roundtrip-{}",
        chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)
    ));
    std::fs::create_dir_all(&dir).unwrap();

    // Build a project.
    let mut tl = Timeline::empty();
    let mut log = OpLog::new();
    let aid = "a1".to_string();
    let tid = ids::track();

    let asset = Asset {
        asset_id: aid.clone(),
        uri: "file:///a.mp4".into(),
        duration_sec: 60.0,
        has_video: true,
        has_audio: true,
        fps: Some(30.0),
        resolution: Some(Resolution { w: 1920, h: 1080 }),
        transcript_ref: None,
        shot_list_ref: None,
    };

    for op in [
        Op::new(OpKind::AddAsset(asset)),
        Op::new(OpKind::AddTrack {
            track_id: tid.clone(),
            kind: TrackKind::Video,
        }),
        Op::new(OpKind::InsertClip {
            track_id: tid.clone(),
            clip: ClipItem {
                item_id: "c1".into(),
                asset_id: aid.clone(),
                src_in: 0.0,
                src_out: 5.0,
                timeline_in: 0.0,
                timeline_out: 5.0,
                speed: 1.0,
                effects: vec![],
                markers: vec![],
                metadata: ClipMetadata::default(),
            },
        }),
        Op::new(OpKind::PinClip {
            track_id: tid.clone(),
            item_id: "c1".into(),
        }),
        Op::new(OpKind::InsertClip {
            track_id: tid.clone(),
            clip: ClipItem {
                item_id: "c2".into(),
                asset_id: aid,
                src_in: 10.0,
                src_out: 15.0,
                timeline_in: 5.0,
                timeline_out: 10.0,
                speed: 1.0,
                effects: vec![],
                markers: vec![],
                metadata: ClipMetadata::default(),
            },
        }),
    ] {
        slop_core::reducer::apply(&mut tl, &op).unwrap();
        log.push(op);
    }

    // Persist and reload.
    let ops_path = dir.join("ops.jsonl");
    log.save(&ops_path).unwrap();
    let reloaded = OpLog::load(&ops_path).unwrap();
    let rebuilt = slop_core::reducer::replay(reloaded.ops()).unwrap();

    assert_eq!(rebuilt, tl);

    // Validate semantics on the rebuilt timeline.
    slop_core::validator::validate_timeline_semantics(&rebuilt).unwrap();
    let json = serde_json::to_value(&rebuilt).unwrap();
    slop_core::validator::validate_timeline_schema(&json).unwrap();

    // Confirm pin survived the round trip.
    let track = rebuilt.track(&tid).unwrap();
    let TrackItem::Clip(c1) = track.items.iter().find(|i| i.id() == "c1").unwrap() else {
        panic!("expected clip");
    };
    assert!(c1.metadata.locked_by_user);
}
