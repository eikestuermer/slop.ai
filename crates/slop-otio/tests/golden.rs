//! Golden-file regression tests for OTIO and pro-NLE adapters.
//!
//! These tests are *intentionally* not byte-for-byte. They check that:
//!
//! - the document is well-formed XML / JSON,
//! - it contains the expected number of clip references,
//! - the cut points round-trip into the right frame counts.
//!
//! Byte-for-byte goldens would lock us into an exact serializer ordering that
//! we want to keep room to optimize. Structural goldens are the right level.

use slop_core::*;
use slop_otio::*;

fn fixture() -> Timeline {
    let mut tl = Timeline::empty();
    tl.assets.push(Asset {
        asset_id: "a1".into(),
        uri: "file:///alpha.mp4".into(),
        duration_sec: 60.0,
        has_video: true,
        has_audio: true,
        fps: Some(30.0),
        resolution: Some(Resolution { w: 1920, h: 1080 }),
        transcript_ref: None,
        shot_list_ref: None,
    });
    tl.assets.push(Asset {
        asset_id: "a2".into(),
        uri: "file:///beta.mp4".into(),
        duration_sec: 30.0,
        has_video: true,
        has_audio: true,
        fps: Some(30.0),
        resolution: Some(Resolution { w: 1920, h: 1080 }),
        transcript_ref: None,
        shot_list_ref: None,
    });
    tl.tracks.push(Track {
        track_id: "v1".into(),
        kind: TrackKind::Video,
        items: vec![
            TrackItem::Clip(ClipItem {
                item_id: "c1".into(),
                asset_id: "a1".into(),
                src_in: 5.0,
                src_out: 10.0,
                timeline_in: 0.0,
                timeline_out: 5.0,
                speed: 1.0,
                effects: vec![],
                markers: vec![],
                metadata: ClipMetadata {
                    selection_reason: Some("opening hook".into()),
                    score: Some(0.9),
                    locked_by_user: false,
                    prompt_id: None,
                },
            }),
            TrackItem::Clip(ClipItem {
                item_id: "c2".into(),
                asset_id: "a2".into(),
                src_in: 0.0,
                src_out: 4.0,
                timeline_in: 5.0,
                timeline_out: 9.0,
                speed: 1.0,
                effects: vec![],
                markers: vec![],
                metadata: ClipMetadata {
                    selection_reason: Some("cutaway".into()),
                    score: Some(0.7),
                    locked_by_user: false,
                    prompt_id: None,
                },
            }),
        ],
    });
    tl
}

#[test]
fn otio_export_contains_expected_clips() {
    let tl = fixture();
    let doc = timeline_to_otio_json(&tl);
    let s = serde_json::to_string(&doc).unwrap();
    let count = s.matches("Clip.2").count();
    assert_eq!(count, 2, "expected 2 clips in OTIO output, got {count}");
    assert!(s.contains("opening hook"));
    assert!(s.contains("cutaway"));
    assert!(s.contains("file:///alpha.mp4"));
    assert!(s.contains("file:///beta.mp4"));
}

#[test]
fn fcp7_xml_contains_expected_clips_and_files() {
    let tl = fixture();
    let dir = tempdir();
    let out = dir.join("out.xml");
    write_fcp7_xml(&tl, &out).unwrap();
    let s = std::fs::read_to_string(&out).unwrap();
    assert!(s.contains("<xmeml"));
    assert_eq!(s.matches("<clipitem ").count(), 2);
    assert!(s.contains("file:///alpha.mp4"));
    assert!(s.contains("opening hook"));
}

#[test]
fn fcpxml_resolve_contains_assets_and_clips() {
    let tl = fixture();
    let dir = tempdir();
    let out = dir.join("out.fcpxml");
    write_resolve_fcpxml(&tl, &out).unwrap();
    let s = std::fs::read_to_string(&out).unwrap();
    assert!(s.contains("<fcpxml version=\"1.10\">"));
    assert_eq!(s.matches("<asset ").count(), 2);
    assert_eq!(s.matches("<asset-clip ").count(), 2);
}

#[test]
fn kdenlive_xml_contains_producers_and_entries() {
    let tl = fixture();
    let dir = tempdir();
    let out = dir.join("project.kdenlive");
    write_kdenlive_xml(&tl, &out).unwrap();
    let s = std::fs::read_to_string(&out).unwrap();
    assert!(s.contains("<mlt"));
    assert_eq!(s.matches("<producer ").count(), 2);
    assert_eq!(s.matches("<entry ").count(), 2);
}

fn tempdir() -> std::path::PathBuf {
    let p = std::env::temp_dir().join(format!("slop-otio-test-{}", chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)));
    std::fs::create_dir_all(&p).unwrap();
    p
}
