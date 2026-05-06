//! Premiere Pro FCP7 XML adapter.
//!
//! This is the format Adobe Premiere has accepted for over a decade. It is
//! the de-facto safe interchange path for rough cuts.
//!
//! The output models:
//!
//! - one `<sequence>` per timeline,
//! - one `<track>` per Slop track,
//! - one `<clipitem>` per Slop clip with `<file>` references,
//! - markers as `<marker>` children of `<clipitem>`,
//! - timeline-level captions written as a sidecar SRT (Premiere does not
//!   round-trip caption tracks via FCP7 XML reliably).

use slop_core::{Timeline, TrackItem, TrackKind};
use std::path::Path;

/// Write a Premiere-compatible FCP7 XML document for `tl` to `out`.
pub fn write_fcp7_xml(tl: &Timeline, out: &Path) -> std::io::Result<()> {
    let fps = tl.project.fps;
    let timebase = fps.round() as u32;
    let ntsc = (fps - timebase as f64).abs() > 0.01;

    let mut xml = String::new();
    xml.push_str(r#"<?xml version="1.0" encoding="UTF-8"?>"#);
    xml.push('\n');
    xml.push_str(r#"<!DOCTYPE xmeml>"#);
    xml.push('\n');
    xml.push_str(r#"<xmeml version="5">"#);
    xml.push('\n');
    xml.push_str("  <sequence>\n");
    xml.push_str("    <name>Slop AI Rough Cut</name>\n");
    let total = (tl.duration_sec() * fps).round() as i64;
    xml.push_str(&format!("    <duration>{}</duration>\n", total));
    xml.push_str("    <rate>\n");
    xml.push_str(&format!("      <timebase>{}</timebase>\n", timebase));
    xml.push_str(&format!(
        "      <ntsc>{}</ntsc>\n",
        if ntsc { "TRUE" } else { "FALSE" }
    ));
    xml.push_str("    </rate>\n");
    xml.push_str("    <media>\n");

    for kind in [TrackKind::Video, TrackKind::Audio] {
        let kind_label = match kind {
            TrackKind::Video => "video",
            TrackKind::Audio => "audio",
        };
        xml.push_str(&format!("      <{}>\n", kind_label));
        if matches!(kind, TrackKind::Video) {
            xml.push_str("        <format><samplecharacteristics>");
            xml.push_str(&format!(
                "<width>{}</width><height>{}</height>",
                tl.project.resolution.w, tl.project.resolution.h
            ));
            xml.push_str("</samplecharacteristics></format>\n");
        }
        for track in tl.tracks.iter().filter(|t| t.kind == kind) {
            xml.push_str("        <track>\n");
            for item in &track.items {
                if let TrackItem::Clip(c) = item {
                    let asset = tl.asset(&c.asset_id);
                    let media_dur = asset.map(|a| a.duration_sec).unwrap_or(c.src_out);
                    let uri = asset.map(|a| a.uri.clone()).unwrap_or_default();
                    let in_f = (c.src_in * fps).round() as i64;
                    let out_f = (c.src_out * fps).round() as i64;
                    let s_f = (c.timeline_in * fps).round() as i64;
                    let e_f = (c.timeline_out * fps).round() as i64;
                    xml.push_str(&format!(
                        "          <clipitem id=\"{}\">\n",
                        escape_xml(&c.item_id)
                    ));
                    xml.push_str(&format!(
                        "            <name>{}</name>\n",
                        escape_xml(c.metadata.selection_reason.as_deref().unwrap_or(&c.item_id))
                    ));
                    xml.push_str(&format!(
                        "            <duration>{}</duration>\n",
                        (media_dur * fps).round() as i64
                    ));
                    xml.push_str(&format!("            <in>{in_f}</in>\n"));
                    xml.push_str(&format!("            <out>{out_f}</out>\n"));
                    xml.push_str(&format!("            <start>{s_f}</start>\n"));
                    xml.push_str(&format!("            <end>{e_f}</end>\n"));
                    xml.push_str(&format!(
                        "            <file id=\"file_{}\">\n",
                        escape_xml(&c.asset_id)
                    ));
                    xml.push_str(&format!(
                        "              <pathurl>{}</pathurl>\n",
                        escape_xml(&uri)
                    ));
                    xml.push_str("            </file>\n");
                    for m in &c.markers {
                        let mf = (m.time_sec * fps).round() as i64;
                        xml.push_str("            <marker>\n");
                        xml.push_str(&format!(
                            "              <name>{}</name>\n",
                            escape_xml(&m.label)
                        ));
                        xml.push_str(&format!("              <in>{mf}</in>\n"));
                        xml.push_str(&format!("              <out>{mf}</out>\n"));
                        xml.push_str("            </marker>\n");
                    }
                    xml.push_str("          </clipitem>\n");
                }
            }
            xml.push_str("        </track>\n");
        }
        xml.push_str(&format!("      </{}>\n", kind_label));
    }

    xml.push_str("    </media>\n");
    xml.push_str("  </sequence>\n");
    xml.push_str("</xmeml>\n");

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
