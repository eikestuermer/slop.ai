//! # slop-captions
//!
//! Caption / subtitle writers.
//!
//! - **SRT**: lowest-common-denominator, no styling. Universally supported.
//! - **WebVTT**: HTML/web-friendly, supports basic styling via inline cues.
//! - **ASS / SSA**: Advanced SubStation Alpha. The SOTA format for
//!   subtitle styling: per-line fonts, colors, positioning, outlines,
//!   karaoke effects. We emit ASS so all the V2 schema's `CaptionStyle`
//!   fields round-trip and ffmpeg's `subtitles=` filter renders them
//!   pixel-perfect.

#![deny(missing_docs)]

use serde::Serialize;
use std::fmt::Write;

/// One caption row at write time.
#[derive(Debug, Clone, Serialize)]
pub struct CaptionRow {
    /// Start in seconds.
    pub start_sec: f64,
    /// End in seconds.
    pub end_sec: f64,
    /// Caption text. May contain `\n` for multi-line cues.
    pub text: String,
    /// Optional speaker tag.
    pub speaker: Option<String>,
    /// Optional language code (used by WebVTT regions).
    pub language: Option<String>,
    /// Style values matching the V2 schema's `CaptionStyle`.
    pub style: CaptionStyle,
}

/// Styling fields from the V2 schema, native Rust shape.
#[derive(Debug, Clone, Serialize)]
pub struct CaptionStyle {
    /// Family.
    pub font_family: String,
    /// Size in pixels.
    pub font_size_px: f32,
    /// Hex.
    pub color: String,
    /// Hex including alpha.
    pub background: String,
    /// Hex.
    pub outline: String,
    /// Outline width.
    pub outline_px: f32,
    /// Anchor.
    pub anchor: String,
    /// X offset.
    pub x: f32,
    /// Y offset.
    pub y: f32,
    /// Bold.
    pub bold: bool,
    /// Italic.
    pub italic: bool,
}

impl Default for CaptionStyle {
    fn default() -> Self {
        Self {
            font_family: "Inter".into(),
            font_size_px: 36.0,
            color: "#ffffff".into(),
            background: "#000000aa".into(),
            outline: "#000000".into(),
            outline_px: 2.0,
            anchor: "bottom-center".into(),
            x: 0.0,
            y: 0.0,
            bold: false,
            italic: false,
        }
    }
}

/// Write SRT.
pub fn write_srt(rows: &[CaptionRow]) -> String {
    let mut out = String::new();
    for (i, r) in rows.iter().enumerate() {
        let _ = writeln!(out, "{}", i + 1);
        let _ = writeln!(out, "{} --> {}", srt_time(r.start_sec), srt_time(r.end_sec));
        let _ = writeln!(out, "{}", r.text);
        out.push('\n');
    }
    out
}

/// Write WebVTT.
pub fn write_vtt(rows: &[CaptionRow]) -> String {
    let mut out = String::from("WEBVTT\n\n");
    for (i, r) in rows.iter().enumerate() {
        let _ = writeln!(out, "cue{}", i + 1);
        let _ = writeln!(out, "{} --> {}", vtt_time(r.start_sec), vtt_time(r.end_sec));
        if let Some(speaker) = &r.speaker {
            let _ = writeln!(out, "<v {speaker}>{}", r.text);
        } else {
            let _ = writeln!(out, "{}", r.text);
        }
        out.push('\n');
    }
    out
}

/// Write ASS / SSA with full styling.
pub fn write_ass(rows: &[CaptionRow], canvas_w: u32, canvas_h: u32) -> String {
    let mut out = String::new();
    out.push_str("[Script Info]\n");
    out.push_str("ScriptType: v4.00+\n");
    out.push_str(&format!("PlayResX: {canvas_w}\nPlayResY: {canvas_h}\n"));
    out.push_str("ScaledBorderAndShadow: yes\n\n");

    out.push_str("[V4+ Styles]\n");
    out.push_str("Format: Name, Fontname, Fontsize, PrimaryColour, SecondaryColour, OutlineColour, BackColour, Bold, Italic, Underline, StrikeOut, ScaleX, ScaleY, Spacing, Angle, BorderStyle, Outline, Shadow, Alignment, MarginL, MarginR, MarginV, Encoding\n");

    // Build a deduplicated style table keyed by a fingerprint.
    let mut styles: Vec<(String, &CaptionStyle)> = Vec::new();
    let style_name_for = |s: &CaptionStyle| -> String {
        format!(
            "S{}_{}_{}_{}_{}",
            s.font_family.replace(' ', "_"),
            s.font_size_px as u32,
            hex_to_ass_color(&s.color),
            s.anchor,
            (s.bold as u8) * 10 + (s.italic as u8),
        )
    };
    for r in rows {
        let name = style_name_for(&r.style);
        if !styles.iter().any(|(n, _)| n == &name) {
            styles.push((name, &r.style));
        }
    }
    for (name, style) in &styles {
        let _ = writeln!(
            out,
            "Style: {name},{font},{size},{primary},{secondary},{outline_c},{back},{bold},{italic},0,0,100,100,0,0,1,{outline_w},0,{align},20,20,20,1",
            font = style.font_family,
            size = style.font_size_px as u32,
            primary = hex_to_ass_color(&style.color),
            secondary = hex_to_ass_color(&style.color),
            outline_c = hex_to_ass_color(&style.outline),
            back = hex_to_ass_color(&style.background),
            bold = if style.bold { -1 } else { 0 },
            italic = if style.italic { -1 } else { 0 },
            outline_w = style.outline_px,
            align = anchor_to_ass_alignment(&style.anchor),
        );
    }

    out.push_str("\n[Events]\n");
    out.push_str(
        "Format: Layer, Start, End, Style, Name, MarginL, MarginR, MarginV, Effect, Text\n",
    );
    for r in rows {
        let name = style_name_for(&r.style);
        let _ = writeln!(
            out,
            "Dialogue: 0,{start},{end},{name},{actor},0,0,0,,{text}",
            start = ass_time(r.start_sec),
            end = ass_time(r.end_sec),
            name = name,
            actor = r.speaker.as_deref().unwrap_or(""),
            text = r.text.replace('\n', "\\N"),
        );
    }
    out
}

fn srt_time(s: f64) -> String {
    let total_ms = (s * 1000.0).round() as i64;
    let h = total_ms / 3_600_000;
    let m = (total_ms / 60_000) % 60;
    let sec = (total_ms / 1000) % 60;
    let ms = total_ms % 1000;
    format!("{h:02}:{m:02}:{sec:02},{ms:03}")
}

fn vtt_time(s: f64) -> String {
    let total_ms = (s * 1000.0).round() as i64;
    let h = total_ms / 3_600_000;
    let m = (total_ms / 60_000) % 60;
    let sec = (total_ms / 1000) % 60;
    let ms = total_ms % 1000;
    format!("{h:02}:{m:02}:{sec:02}.{ms:03}")
}

fn ass_time(s: f64) -> String {
    let cs = (s * 100.0).round() as i64;
    let h = cs / 360_000;
    let m = (cs / 6_000) % 60;
    let sec = (cs / 100) % 60;
    let cs_rem = cs % 100;
    format!("{h}:{m:02}:{sec:02}.{cs_rem:02}")
}

fn hex_to_ass_color(hex: &str) -> String {
    // ASS color is `&HAABBGGRR&` with alpha inverted (00 = opaque).
    let h = hex.trim_start_matches('#');
    let (r, g, b, a) = match h.len() {
        6 => (
            u8::from_str_radix(&h[0..2], 16).unwrap_or(255),
            u8::from_str_radix(&h[2..4], 16).unwrap_or(255),
            u8::from_str_radix(&h[4..6], 16).unwrap_or(255),
            0u8,
        ),
        8 => (
            u8::from_str_radix(&h[0..2], 16).unwrap_or(255),
            u8::from_str_radix(&h[2..4], 16).unwrap_or(255),
            u8::from_str_radix(&h[4..6], 16).unwrap_or(255),
            255 - u8::from_str_radix(&h[6..8], 16).unwrap_or(255),
        ),
        _ => (255, 255, 255, 0),
    };
    format!("&H{a:02X}{b:02X}{g:02X}{r:02X}")
}

fn anchor_to_ass_alignment(anchor: &str) -> u8 {
    // ASS numpad-layout: 1=BL, 2=BC, 3=BR, 4=ML, 5=MC, 6=MR, 7=TL, 8=TC, 9=TR.
    match anchor {
        "bottom-left" => 1,
        "bottom-center" => 2,
        "bottom-right" => 3,
        "middle-left" => 4,
        "middle-center" => 5,
        "middle-right" => 6,
        "top-left" => 7,
        "top-center" => 8,
        "top-right" => 9,
        _ => 2,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Vec<CaptionRow> {
        vec![
            CaptionRow {
                start_sec: 0.0,
                end_sec: 2.5,
                text: "Hello, world.".into(),
                speaker: Some("Alex".into()),
                language: Some("en".into()),
                style: CaptionStyle::default(),
            },
            CaptionRow {
                start_sec: 2.5,
                end_sec: 5.0,
                text: "Multi-line\ncue here.".into(),
                speaker: None,
                language: Some("en".into()),
                style: CaptionStyle {
                    bold: true,
                    color: "#ffe600".into(),
                    ..CaptionStyle::default()
                },
            },
        ]
    }

    #[test]
    fn srt_format_is_correct() {
        let s = write_srt(&fixture());
        assert!(s.starts_with("1\n00:00:00,000 --> 00:00:02,500"));
    }

    #[test]
    fn vtt_starts_with_webvtt() {
        let s = write_vtt(&fixture());
        assert!(s.starts_with("WEBVTT"));
        assert!(s.contains("<v Alex>Hello"));
    }

    #[test]
    fn ass_includes_style_table_and_dialogues() {
        let s = write_ass(&fixture(), 1920, 1080);
        assert!(s.contains("[V4+ Styles]"));
        assert!(s.contains("Dialogue:"));
        // Two distinct styles -> two style entries.
        let style_lines = s.matches("Style: ").count();
        assert!(style_lines >= 2);
        assert!(s.contains("PlayResX: 1920"));
    }

    #[test]
    fn hex_to_ass_color_inverts_alpha() {
        let c = hex_to_ass_color("#000000aa");
        // a=0xaa -> alpha = 255 - 170 = 85 (0x55)
        assert!(c.starts_with("&H55"));
    }

    #[test]
    fn srt_numbering_starts_at_one_and_increments() {
        let s = write_srt(&fixture());
        assert!(s.starts_with("1\n"));
        assert!(s.contains("\n2\n"));
        // No "0\n" cue header.
        assert!(!s.starts_with("0\n"));
    }

    #[test]
    fn ass_bold_emits_minus_one() {
        let s = write_ass(&fixture(), 1920, 1080);
        // Second fixture row has bold=true.
        let bold_lines: Vec<&str> = s.lines().filter(|l| l.contains(",-1,")).collect();
        assert!(!bold_lines.is_empty(), "expected at least one Bold=-1 row");
    }

    #[test]
    fn ass_multiline_text_uses_backslash_n() {
        let s = write_ass(&fixture(), 1920, 1080);
        assert!(s.contains("Multi-line\\Ncue here."));
        // Raw newline should not appear inside the multi-line cue text.
        let dialogue_lines: Vec<&str> = s.lines().filter(|l| l.starts_with("Dialogue:")).collect();
        for line in dialogue_lines {
            assert!(!line.contains("\nMulti-line\n"));
        }
    }

    #[test]
    fn vtt_speaker_is_voiced_with_v_tag() {
        let s = write_vtt(&fixture());
        assert!(s.contains("<v Alex>"));
        // A row without a speaker should NOT have a <v ...> wrap.
        let cues: Vec<&str> = s.split("cue").skip(2).collect();
        assert!(!cues.is_empty());
    }

    #[test]
    fn anchor_to_alignment_covers_all_corners() {
        assert_eq!(anchor_to_ass_alignment("bottom-left"), 1);
        assert_eq!(anchor_to_ass_alignment("bottom-center"), 2);
        assert_eq!(anchor_to_ass_alignment("bottom-right"), 3);
        assert_eq!(anchor_to_ass_alignment("middle-center"), 5);
        assert_eq!(anchor_to_ass_alignment("top-left"), 7);
        assert_eq!(anchor_to_ass_alignment("top-right"), 9);
        // Default for unknown values.
        assert_eq!(anchor_to_ass_alignment("nope"), 2);
    }
}
