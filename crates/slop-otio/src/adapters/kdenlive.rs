//! Kdenlive native MLT XML adapter.
//!
//! Kdenlive accepts MLT XML projects directly. We emit a minimal MLT
//! document with one tractor (master timeline) referencing one playlist per
//! track, with one producer per asset.

use slop_core::{Timeline, TrackItem, TrackKind};
use std::path::Path;

/// Write a Kdenlive-compatible MLT XML document for `tl` to `out`.
pub fn write_kdenlive_xml(tl: &Timeline, out: &Path) -> std::io::Result<()> {
    let fps = tl.project.fps;
    let mut xml = String::new();
    xml.push_str(r#"<?xml version="1.0" encoding="UTF-8" standalone="no"?>"#);
    xml.push('\n');
    xml.push_str(&format!(
        "<mlt LC_NUMERIC=\"C\" version=\"7.20.0\" producer=\"main_bin\" profile=\"atsc_{w}p{fps_round}\">\n",
        w = tl.project.resolution.w,
        fps_round = fps.round() as u32,
    ));

    xml.push_str(&format!(
        "  <profile description=\"Slop AI default\" width=\"{w}\" height=\"{h}\" frame_rate_num=\"{fps_round}\" frame_rate_den=\"1\"/>\n",
        w = tl.project.resolution.w,
        h = tl.project.resolution.h,
        fps_round = fps.round() as u32,
    ));

    for asset in &tl.assets {
        xml.push_str(&format!(
            "  <producer id=\"prod_{id}\" in=\"00:00:00.000\" out=\"{dur:.3}\">\n",
            id = escape_xml(&asset.asset_id),
            dur = asset.duration_sec,
        ));
        xml.push_str(&format!(
            "    <property name=\"resource\">{}</property>\n",
            escape_xml(asset.uri.strip_prefix("file://").unwrap_or(&asset.uri))
        ));
        xml.push_str("  </producer>\n");
    }

    let mut playlist_ids = Vec::new();
    for track in &tl.tracks {
        let pid = format!("playlist_{}", escape_xml(&track.track_id));
        xml.push_str(&format!("  <playlist id=\"{pid}\">\n"));
        let mut cursor = 0.0_f64;
        for item in &track.items {
            if let TrackItem::Clip(c) = item {
                if c.timeline_in > cursor + 1e-6 {
                    let blank = c.timeline_in - cursor;
                    xml.push_str(&format!(
                        "    <blank length=\"{:.3}\"/>\n",
                        blank
                    ));
                    cursor = c.timeline_in;
                }
                xml.push_str(&format!(
                    "    <entry producer=\"prod_{id}\" in=\"{si:.3}\" out=\"{so:.3}\"/>\n",
                    id = escape_xml(&c.asset_id),
                    si = c.src_in,
                    so = c.src_out,
                ));
                cursor = c.timeline_out;
            }
        }
        xml.push_str("  </playlist>\n");
        playlist_ids.push((pid, track.kind));
    }

    xml.push_str(&format!(
        "  <tractor id=\"main\" in=\"00:00:00.000\" out=\"{dur:.3}\">\n",
        dur = tl.duration_sec()
    ));
    for (pid, kind) in &playlist_ids {
        let hide = match kind {
            TrackKind::Video => "audio",
            TrackKind::Audio => "video",
        };
        xml.push_str(&format!(
            "    <track producer=\"{pid}\" hide=\"{hide}\"/>\n"
        ));
    }
    xml.push_str("  </tractor>\n");
    xml.push_str("</mlt>\n");

    if let Some(p) = out.parent() {
        if !p.as_os_str().is_empty() {
            std::fs::create_dir_all(p)?;
        }
    }
    std::fs::write(out, xml)
}

fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}
