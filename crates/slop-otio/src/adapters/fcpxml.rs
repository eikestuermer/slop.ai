//! DaVinci Resolve FCPXML adapter.
//!
//! Resolve 17+ accepts native OTIO; for older Resolve versions and as a
//! fallback, FCPXML 1.10 is the safest interchange. We emit the smallest
//! valid document that carries cuts and references.

use slop_core::{Timeline, TrackItem};
use std::path::Path;

/// Write an FCPXML 1.10 document for `tl` to `out`. Suitable as a Resolve
/// fallback when OTIO is not available.
pub fn write_resolve_fcpxml(tl: &Timeline, out: &Path) -> std::io::Result<()> {
    let fps = tl.project.fps;
    let timebase = fps.round() as u32;
    let frame_dur = format!("1/{timebase}s");

    let mut xml = String::new();
    xml.push_str(r#"<?xml version="1.0" encoding="UTF-8"?>"#);
    xml.push('\n');
    xml.push_str(r#"<!DOCTYPE fcpxml>"#);
    xml.push('\n');
    xml.push_str(r#"<fcpxml version="1.10">"#);
    xml.push('\n');
    xml.push_str("  <resources>\n");
    xml.push_str(&format!(
        "    <format id=\"r0\" name=\"FFVideoFormat{w}p{fps_round}\" frameDuration=\"{fd}\" width=\"{w}\" height=\"{h}\"/>\n",
        w = tl.project.resolution.w,
        h = tl.project.resolution.h,
        fps_round = timebase,
        fd = frame_dur,
    ));
    for (i, asset) in tl.assets.iter().enumerate() {
        xml.push_str(&format!(
            "    <asset id=\"a{i}\" name=\"{name}\" src=\"{uri}\" duration=\"{dur}s\" hasVideo=\"{hv}\" hasAudio=\"{ha}\"/>\n",
            i = i,
            name = escape_xml(&asset.asset_id),
            uri = escape_xml(&asset.uri),
            dur = asset.duration_sec,
            hv = if asset.has_video { "1" } else { "0" },
            ha = if asset.has_audio { "1" } else { "0" },
        ));
    }
    xml.push_str("  </resources>\n");
    xml.push_str("  <library>\n");
    xml.push_str("    <event name=\"Slop AI\">\n");
    xml.push_str(&format!(
        "      <project name=\"Slop AI Rough Cut\">\n        <sequence format=\"r0\" duration=\"{}s\">\n",
        tl.duration_sec()
    ));
    xml.push_str("          <spine>\n");
    let asset_idx = |id: &str| -> Option<usize> {
        tl.assets.iter().position(|a| a.asset_id == id)
    };
    for track in &tl.tracks {
        for item in &track.items {
            if let TrackItem::Clip(c) = item {
                if let Some(i) = asset_idx(&c.asset_id) {
                    xml.push_str(&format!(
                        "            <asset-clip name=\"{name}\" ref=\"a{i}\" offset=\"{off}s\" start=\"{si}s\" duration=\"{dur}s\"/>\n",
                        name = escape_xml(c.metadata.selection_reason.as_deref().unwrap_or(&c.item_id)),
                        off = c.timeline_in,
                        si = c.src_in,
                        dur = c.src_out - c.src_in,
                    ));
                }
            }
        }
    }
    xml.push_str("          </spine>\n");
    xml.push_str("        </sequence>\n      </project>\n");
    xml.push_str("    </event>\n  </library>\n</fcpxml>\n");

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
