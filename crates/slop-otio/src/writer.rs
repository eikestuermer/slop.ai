//! Convert a `slop_core::Timeline` to OTIO and write it to disk.

use crate::schema::*;
use serde_json::json;
use slop_core::{Timeline as SlopTimeline, TrackItem, TrackKind};
use std::path::Path;

/// Build the OTIO JSON document for `tl` without writing to disk.
pub fn timeline_to_otio_json(tl: &SlopTimeline) -> Timeline {
    let fps = tl.project.fps;
    let mut otio_tracks = Vec::new();

    for track in &tl.tracks {
        let kind_str = match track.kind {
            TrackKind::Video => "Video",
            TrackKind::Audio => "Audio",
        };
        let mut children: Vec<TrackChild> = Vec::new();
        let mut cursor = 0.0_f64;
        for item in &track.items {
            // Insert gap if necessary to fill the time before this item.
            if item.timeline_in() > cursor + 1e-6 {
                let gap_dur = item.timeline_in() - cursor;
                children.push(TrackChild::Gap(Gap {
                    otio_schema: "Gap.1",
                    name: "gap".to_string(),
                    source_range: TimeRange::from_secs(0.0, gap_dur, fps),
                }));
            }
            match item {
                TrackItem::Clip(c) => {
                    let asset = tl.asset(&c.asset_id);
                    let media_dur = asset.map(|a| a.duration_sec).unwrap_or(c.src_out);
                    let uri = asset.map(|a| a.uri.clone()).unwrap_or_default();
                    let mut markers = Vec::new();
                    for m in &c.markers {
                        markers.push(Marker {
                            otio_schema: "Marker.1",
                            name: m.label.clone(),
                            color: marker_color(&m.color),
                            marked_range: TimeRange::from_secs(m.time_sec, m.time_sec, fps),
                            metadata: json!({}),
                        });
                    }
                    let mut metadata = json!({});
                    if let Ok(meta) = serde_json::to_value(&c.metadata) {
                        metadata = json!({ "slop": meta });
                    }
                    children.push(TrackChild::Clip(Clip {
                        otio_schema: "Clip.2",
                        name: c
                            .metadata
                            .selection_reason
                            .clone()
                            .unwrap_or_else(|| c.asset_id.clone()),
                        source_range: TimeRange::from_secs(c.src_in, c.src_out, fps),
                        media_reference: ExternalReference {
                            otio_schema: "ExternalReference.1",
                            target_url: uri,
                            available_range: TimeRange::from_secs(0.0, media_dur, fps),
                            metadata: json!({}),
                        },
                        markers,
                        effects: Vec::new(),
                        metadata,
                    }));
                    cursor = c.timeline_out;
                }
                TrackItem::Gap(g) => {
                    children.push(TrackChild::Gap(Gap {
                        otio_schema: "Gap.1",
                        name: "gap".to_string(),
                        source_range: TimeRange::from_secs(
                            0.0,
                            g.timeline_out - g.timeline_in,
                            fps,
                        ),
                    }));
                    cursor = g.timeline_out;
                }
            }
        }

        otio_tracks.push(Track {
            otio_schema: "Track.1",
            name: track.track_id.clone(),
            kind: kind_str.to_string(),
            children,
            metadata: json!({}),
        });
    }

    Timeline {
        otio_schema: "Timeline.1",
        name: "Slop AI Rough Cut".to_string(),
        global_start_time: RationalTime::from_secs(0.0, fps),
        tracks: Stack {
            otio_schema: "Stack.1",
            name: "tracks".to_string(),
            children: otio_tracks,
            metadata: json!({}),
        },
        metadata: json!({
            "slop": {
                "exporter_version": env!("CARGO_PKG_VERSION"),
                "captions": tl.captions,
            }
        }),
    }
}

/// Write OTIO JSON for `tl` to `out`.
pub fn write_otio(tl: &SlopTimeline, out: &Path) -> std::io::Result<()> {
    let doc = timeline_to_otio_json(tl);
    let s = serde_json::to_string_pretty(&doc)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    if let Some(parent) = out.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
        }
    }
    std::fs::write(out, s)
}

fn marker_color(hex: &str) -> String {
    // OTIO uses named colors; map our hex strings to nearest names.
    match hex {
        "#5aef9e" | "#0f0" => "GREEN".into(),
        "#f0c050" | "#ff0" => "YELLOW".into(),
        "#ff7070" | "#f00" => "RED".into(),
        _ => "BLUE".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::timeline_to_otio_json;
    use crate::schema::TrackChild;
    use slop_core::*;

    fn fixture() -> Timeline {
        let mut tl = Timeline::empty();
        tl.assets.push(Asset {
            asset_id: "a1".into(),
            uri: "file:///x.mp4".into(),
            duration_sec: 60.0,
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
            items: vec![TrackItem::Clip(ClipItem {
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
                    selection_reason: Some("strong opening".into()),
                    score: Some(0.91),
                    locked_by_user: false,
                    prompt_id: None,
                },
            })],
        });
        tl
    }

    #[test]
    fn writes_valid_otio_json() {
        let tl = fixture();
        let doc = timeline_to_otio_json(&tl);
        let s = serde_json::to_string(&doc).unwrap();
        assert!(s.contains("Timeline.1"));
        assert!(s.contains("Track.1"));
        assert!(s.contains("Clip.2"));
        assert!(s.contains("ExternalReference.1"));
        assert!(s.contains("strong opening"));
    }

    #[test]
    fn write_otio_round_trips_through_serde_parse() {
        let tl = fixture();
        let dir = std::env::temp_dir().join(format!(
            "slop-otio-rt-{}",
            chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let out = dir.join("rt.otio");
        super::write_otio(&tl, &out).unwrap();
        let raw = std::fs::read_to_string(&out).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&raw).unwrap();
        assert_eq!(parsed["OTIO_SCHEMA"], "Timeline.1");
        assert_eq!(
            parsed["tracks"]["children"][0]["children"]
                .as_array()
                .unwrap()
                .len(),
            1
        );
    }

    #[test]
    fn gap_inserted_when_clip_starts_after_zero() {
        let mut tl = fixture();
        // Move the clip so it starts at 5.0 with a 5s leading gap.
        if let TrackItem::Clip(c) = &mut tl.tracks[0].items[0] {
            c.timeline_in = 5.0;
            c.timeline_out = 10.0;
        }
        let doc = timeline_to_otio_json(&tl);
        let children = doc.tracks.children[0].children.clone();
        assert!(matches!(children[0], TrackChild::Gap(_)));
        if let TrackChild::Gap(g) = &children[0] {
            assert!((g.source_range.duration.value - 5.0 * 30.0).abs() < 1e-6);
        }
    }
}
