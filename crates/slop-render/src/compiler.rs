//! Compile a `Timeline` to a list of FFmpeg `-i` inputs and a `-filter_complex`
//! string.

use serde::{Deserialize, Serialize};
use slop_core::{Timeline, TrackItem, TrackKind};
use std::collections::BTreeMap;

/// Knobs controlling render output.
#[derive(Debug, Clone)]
pub struct RenderOptions {
    /// Target output width.
    pub width: u32,
    /// Target output height.
    pub height: u32,
    /// Output frame rate. Falls back to the project's fps if `None`.
    pub fps: Option<f64>,
    /// libx264 CRF.
    pub crf: u32,
    /// libx264 preset.
    pub preset: &'static str,
    /// AAC bitrate, e.g. `"192k"`.
    pub audio_bitrate: &'static str,
    /// If `true`, burn captions in via `drawtext` filter. If `false`,
    /// captions are emitted as a sidecar SRT only.
    pub burn_captions: bool,
}

impl Default for RenderOptions {
    fn default() -> Self {
        Self {
            width: 1920,
            height: 1080,
            fps: None,
            crf: 20,
            preset: "veryfast",
            audio_bitrate: "192k",
            burn_captions: false,
        }
    }
}

/// The compiler's output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompiledRender {
    /// `-i` inputs in order. Each entry is the URI/path to feed FFmpeg.
    pub inputs: Vec<String>,
    /// `-filter_complex` string.
    pub filtergraph: String,
    /// Final mapped video label (e.g. `"[vout]"`).
    pub video_out: String,
    /// Final mapped audio label, or `None` if the timeline has no audio.
    pub audio_out: Option<String>,
    /// Suggested encoder args (`-c:v libx264 -crf 20 -preset veryfast ...`).
    pub encoder_args: Vec<String>,
    /// Optional sidecar SRT content, if captions exist and `burn_captions`
    /// was false.
    pub sidecar_srt: Option<String>,
    /// Output frame rate.
    pub fps: f64,
}

/// Compile a Timeline. Does *not* invoke ffmpeg; see [`crate::runner::render`].
pub fn compile_timeline(tl: &Timeline, opts: &RenderOptions) -> CompiledRender {
    let fps = opts.fps.unwrap_or(tl.project.fps);

    // Stable input ordering: every distinct asset_id used by any track gets
    // exactly one `-i` slot.
    let mut input_index: BTreeMap<String, usize> = BTreeMap::new();
    let mut inputs: Vec<String> = Vec::new();
    for track in &tl.tracks {
        for item in &track.items {
            if let TrackItem::Clip(c) = item {
                if !input_index.contains_key(&c.asset_id) {
                    let idx = inputs.len();
                    if let Some(asset) = tl.asset(&c.asset_id) {
                        inputs.push(asset.uri.clone());
                    } else {
                        inputs.push(format!("MISSING:{}", c.asset_id));
                    }
                    input_index.insert(c.asset_id.clone(), idx);
                }
            }
        }
    }

    let mut graph = String::new();
    let mut video_track_labels: Vec<String> = Vec::new();
    let mut audio_track_labels: Vec<String> = Vec::new();

    for (ti, track) in tl.tracks.iter().enumerate() {
        let mut clip_v_labels: Vec<String> = Vec::new();
        let mut clip_a_labels: Vec<String> = Vec::new();

        for (ci, item) in track.items.iter().enumerate() {
            let TrackItem::Clip(c) = item else { continue };
            let Some(&iidx) = input_index.get(&c.asset_id) else {
                continue;
            };

            if matches!(track.kind, TrackKind::Video) {
                let lbl = format!("v_t{ti}_c{ci}");
                graph.push_str(&format!(
                    "[{iidx}:v]trim=start={si}:end={so},setpts=PTS-STARTPTS,scale={w}:{h}:force_original_aspect_ratio=decrease,pad={w}:{h}:(ow-iw)/2:(oh-ih)/2,setsar=1[{lbl}];",
                    iidx = iidx,
                    si = c.src_in,
                    so = c.src_out,
                    w = opts.width,
                    h = opts.height,
                    lbl = lbl
                ));
                // Apply per-clip fades.
                let mut current = lbl.clone();
                for eff in &c.effects {
                    match eff.kind {
                        slop_core::EffectKind::FadeIn => {
                            let dur = eff.duration_sec.unwrap_or(0.5);
                            let next = format!("{current}_fi");
                            graph.push_str(&format!("[{current}]fade=t=in:st=0:d={dur}[{next}];"));
                            current = next;
                        }
                        slop_core::EffectKind::FadeOut => {
                            let dur = eff.duration_sec.unwrap_or(0.5);
                            let clip_dur = c.src_out - c.src_in;
                            let st = (clip_dur - dur).max(0.0);
                            let next = format!("{current}_fo");
                            graph.push_str(&format!(
                                "[{current}]fade=t=out:st={st}:d={dur}[{next}];"
                            ));
                            current = next;
                        }
                        slop_core::EffectKind::CrossDissolve => {
                            // Approximated: overlap with neighbor handled in
                            // future revision. V1 ignores cross-dissolve at
                            // render time and treats it as a hard cut.
                        }
                    }
                }
                clip_v_labels.push(current);
            } else {
                // Audio track.
                let lbl = format!("a_t{ti}_c{ci}");
                graph.push_str(&format!(
                    "[{iidx}:a]atrim=start={si}:end={so},asetpts=PTS-STARTPTS[{lbl}];",
                    iidx = iidx,
                    si = c.src_in,
                    so = c.src_out,
                    lbl = lbl
                ));
                let mut current = lbl.clone();
                for eff in &c.effects {
                    match eff.kind {
                        slop_core::EffectKind::FadeIn => {
                            let dur = eff.duration_sec.unwrap_or(0.5);
                            let next = format!("{current}_afi");
                            graph.push_str(&format!("[{current}]afade=t=in:st=0:d={dur}[{next}];"));
                            current = next;
                        }
                        slop_core::EffectKind::FadeOut => {
                            let dur = eff.duration_sec.unwrap_or(0.5);
                            let clip_dur = c.src_out - c.src_in;
                            let st = (clip_dur - dur).max(0.0);
                            let next = format!("{current}_afo");
                            graph.push_str(&format!(
                                "[{current}]afade=t=out:st={st}:d={dur}[{next}];"
                            ));
                            current = next;
                        }
                        slop_core::EffectKind::CrossDissolve => {}
                    }
                }
                clip_a_labels.push(current);
            }
        }

        // Concat per track.
        if !clip_v_labels.is_empty() {
            let n = clip_v_labels.len();
            let inputs: String = clip_v_labels.iter().map(|l| format!("[{l}]")).collect();
            let out = format!("track_v{ti}");
            graph.push_str(&format!("{inputs}concat=n={n}:v=1:a=0[{out}];"));
            video_track_labels.push(out);
        }
        if !clip_a_labels.is_empty() {
            let n = clip_a_labels.len();
            let inputs: String = clip_a_labels.iter().map(|l| format!("[{l}]")).collect();
            let out = format!("track_a{ti}");
            graph.push_str(&format!("{inputs}concat=n={n}:v=0:a=1[{out}];"));
            audio_track_labels.push(out);
        }
    }

    // Compose tracks.
    let video_out = if video_track_labels.is_empty() {
        // No video. Synthesize a black source matching project resolution.
        let lbl = "vout";
        graph.push_str(&format!(
            "color=c=black:s={w}x{h}:r={fps}:d=1[{lbl}];",
            w = opts.width,
            h = opts.height,
            fps = fps,
            lbl = lbl
        ));
        format!("[{lbl}]")
    } else if video_track_labels.len() == 1 {
        format!("[{}]", video_track_labels[0])
    } else {
        // Overlay subsequent tracks onto the first.
        let mut current = video_track_labels[0].clone();
        for (i, next) in video_track_labels.iter().enumerate().skip(1) {
            let out = format!("vmix{i}");
            graph.push_str(&format!("[{current}][{next}]overlay=shortest=0[{out}];"));
            current = out;
        }
        // Final relabel.
        graph.push_str(&format!("[{current}]copy[vout];"));
        "[vout]".to_string()
    };

    let audio_out = if audio_track_labels.is_empty() {
        None
    } else if audio_track_labels.len() == 1 {
        Some(format!("[{}]", audio_track_labels[0]))
    } else {
        let inputs: String = audio_track_labels
            .iter()
            .map(|l| format!("[{l}]"))
            .collect();
        let n = audio_track_labels.len();
        graph.push_str(&format!("{inputs}amix=inputs={n}:duration=longest[aout];"));
        Some("[aout]".to_string())
    };

    // Optional caption burn-in. We append `drawtext` filters chained on the
    // current video out label.
    let mut sidecar_srt = None;
    if !tl.captions.is_empty() {
        if opts.burn_captions {
            let mut current_label = video_out.trim_matches(|c| c == '[' || c == ']').to_string();
            for (i, cap) in tl.captions.iter().enumerate() {
                let next = format!("vcap{i}");
                let safe = escape_drawtext(&cap.text);
                graph.push_str(&format!(
                    "[{current_label}]drawtext=text='{safe}':fontcolor=white:box=1:boxcolor=black@0.5:boxborderw=8:x=(w-text_w)/2:y=h-(text_h*2):enable='between(t,{ti},{to})'[{next}];",
                    ti = cap.timeline_in,
                    to = cap.timeline_out,
                    safe = safe,
                ));
                current_label = next;
            }
            // Re-emit final `[vout]` label.
            graph.push_str(&format!("[{current_label}]copy[vout_capped];"));
        } else {
            sidecar_srt = Some(captions_to_srt(&tl.captions));
        }
    }

    // Trim trailing semicolon for cleanliness.
    if graph.ends_with(';') {
        graph.pop();
    }

    let final_video_out = if !tl.captions.is_empty() && opts.burn_captions {
        "[vout_capped]".to_string()
    } else {
        video_out
    };

    let encoder_args = vec![
        "-c:v".to_string(),
        "libx264".to_string(),
        "-preset".to_string(),
        opts.preset.to_string(),
        "-crf".to_string(),
        opts.crf.to_string(),
        "-pix_fmt".to_string(),
        "yuv420p".to_string(),
        "-movflags".to_string(),
        "+faststart".to_string(),
        "-r".to_string(),
        format!("{fps}"),
        "-c:a".to_string(),
        "aac".to_string(),
        "-b:a".to_string(),
        opts.audio_bitrate.to_string(),
    ];

    CompiledRender {
        inputs,
        filtergraph: graph,
        video_out: final_video_out,
        audio_out,
        encoder_args,
        sidecar_srt,
        fps,
    }
}

fn escape_drawtext(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace(':', "\\:")
        .replace('\'', "\\'")
}

fn captions_to_srt(captions: &[slop_core::Caption]) -> String {
    let mut out = String::new();
    for (i, c) in captions.iter().enumerate() {
        out.push_str(&format!("{}\n", i + 1));
        out.push_str(&format!(
            "{} --> {}\n",
            srt_time(c.timeline_in),
            srt_time(c.timeline_out)
        ));
        out.push_str(&c.text);
        out.push_str("\n\n");
    }
    out
}

fn srt_time(s: f64) -> String {
    let total_ms = (s * 1000.0).round() as i64;
    let h = total_ms / 3_600_000;
    let m = (total_ms / 60_000) % 60;
    let sec = (total_ms / 1000) % 60;
    let ms = total_ms % 1000;
    format!("{:02}:{:02}:{:02},{:03}", h, m, sec, ms)
}

#[cfg(test)]
mod tests {
    use super::*;
    use slop_core::{Asset, ClipItem, Resolution, Track, TrackItem, TrackKind};

    fn fixture_tl() -> Timeline {
        let mut tl = Timeline::empty();
        tl.assets.push(Asset {
            asset_id: "a1".into(),
            uri: "file:///a.mp4".into(),
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
                metadata: Default::default(),
            })],
        });
        tl.tracks.push(Track {
            track_id: "a1t".into(),
            kind: TrackKind::Audio,
            items: vec![TrackItem::Clip(ClipItem {
                item_id: "c2".into(),
                asset_id: "a1".into(),
                src_in: 5.0,
                src_out: 10.0,
                timeline_in: 0.0,
                timeline_out: 5.0,
                speed: 1.0,
                effects: vec![],
                markers: vec![],
                metadata: Default::default(),
            })],
        });
        tl
    }

    #[test]
    fn compiles_simple_timeline() {
        let tl = fixture_tl();
        let r = compile_timeline(&tl, &RenderOptions::default());
        assert_eq!(r.inputs.len(), 1);
        assert_eq!(r.inputs[0], "file:///a.mp4");
        assert!(r.filtergraph.contains("trim=start=5"));
        assert!(r.filtergraph.contains("concat=n=1"));
        assert!(r.audio_out.is_some());
    }

    #[test]
    fn srt_format_is_correct() {
        assert_eq!(srt_time(0.0), "00:00:00,000");
        assert_eq!(srt_time(65.250), "00:01:05,250");
        assert_eq!(srt_time(3661.001), "01:01:01,001");
    }

    #[test]
    fn concat_count_matches_clip_count() {
        let mut tl = fixture_tl();
        // Add a second clip on the same video track.
        if let Some(track) = tl
            .tracks
            .iter_mut()
            .find(|t| matches!(t.kind, TrackKind::Video))
        {
            track.items.push(TrackItem::Clip(ClipItem {
                item_id: "c1b".into(),
                asset_id: "a1".into(),
                src_in: 12.0,
                src_out: 18.0,
                timeline_in: 5.0,
                timeline_out: 11.0,
                speed: 1.0,
                effects: vec![],
                markers: vec![],
                metadata: Default::default(),
            }));
        }
        let r = compile_timeline(&tl, &RenderOptions::default());
        assert!(r.filtergraph.contains("concat=n=2"));
    }

    #[test]
    fn captions_emit_sidecar_srt_when_burn_disabled() {
        let mut tl = fixture_tl();
        tl.captions.push(slop_core::Caption {
            timeline_in: 0.0,
            timeline_out: 2.0,
            text: "hello".into(),
            segment_id: None,
        });
        let opts = RenderOptions {
            burn_captions: false,
            ..RenderOptions::default()
        };
        let r = compile_timeline(&tl, &opts);
        assert!(r.sidecar_srt.is_some());
        let srt = r.sidecar_srt.unwrap();
        assert!(srt.contains("00:00:00,000 --> 00:00:02,000"));
        assert!(srt.contains("hello"));
    }

    #[test]
    fn multiple_video_tracks_emit_overlay() {
        let mut tl = fixture_tl();
        // Add a second video track with a clip.
        tl.tracks.push(Track {
            track_id: "v2".into(),
            kind: TrackKind::Video,
            items: vec![TrackItem::Clip(ClipItem {
                item_id: "c_overlay".into(),
                asset_id: "a1".into(),
                src_in: 1.0,
                src_out: 4.0,
                timeline_in: 0.0,
                timeline_out: 3.0,
                speed: 1.0,
                effects: vec![],
                markers: vec![],
                metadata: Default::default(),
            })],
        });
        let r = compile_timeline(&tl, &RenderOptions::default());
        assert!(r.filtergraph.contains("overlay"));
    }

    #[test]
    fn no_video_tracks_synthesize_black_source() {
        let mut tl = Timeline::empty();
        tl.assets.push(Asset {
            asset_id: "a1".into(),
            uri: "file:///x.mp4".into(),
            duration_sec: 10.0,
            has_video: false,
            has_audio: true,
            fps: None,
            resolution: None,
            transcript_ref: None,
            shot_list_ref: None,
        });
        // Audio-only track.
        tl.tracks.push(Track {
            track_id: "a1t".into(),
            kind: TrackKind::Audio,
            items: vec![TrackItem::Clip(ClipItem {
                item_id: "ca".into(),
                asset_id: "a1".into(),
                src_in: 0.0,
                src_out: 5.0,
                timeline_in: 0.0,
                timeline_out: 5.0,
                speed: 1.0,
                effects: vec![],
                markers: vec![],
                metadata: Default::default(),
            })],
        });
        let r = compile_timeline(&tl, &RenderOptions::default());
        assert!(r.filtergraph.contains("color=c=black"));
        assert_eq!(r.video_out, "[vout]");
    }
}
