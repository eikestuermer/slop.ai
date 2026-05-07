//! Tauri IPC commands. Each is a thin wrapper around the worker crates.
//!
//! Errors are turned into JSON-friendly `String`s on the boundary so the
//! frontend can render them without round-tripping rust enums through
//! serde_json with custom tags.

use crate::state::{AppState, AssetCache, AssetView, JobStatus, ProjectView};
use serde::{Deserialize, Serialize};
use slop_core::{ids, Op, OpKind, TrackKind};
use slop_score::{build_prompt_pack, score_moments, ScoreWeights};
use std::path::PathBuf;
use tauri::State;

type CmdResult<T> = Result<T, String>;

#[tauri::command]
pub async fn load_project(state: State<'_, AppState>, path: String) -> CmdResult<ProjectView> {
    let p = PathBuf::from(&path);
    state.load_from(&p).map_err(|e| e.to_string())?;
    Ok(state.snapshot())
}

#[tauri::command]
pub async fn new_project(state: State<'_, AppState>, path: String) -> CmdResult<ProjectView> {
    let p = PathBuf::from(&path);
    state.load_from(&p).map_err(|e| e.to_string())?;
    // Add a default video and audio track so the planner has a target.
    let v_id = ids::track();
    state
        .apply_and_log(Op::new(OpKind::AddTrack {
            track_id: v_id,
            kind: TrackKind::Video,
        }))
        .map_err(|e| e.to_string())?;
    let a_id = ids::track();
    state
        .apply_and_log(Op::new(OpKind::AddTrack {
            track_id: a_id,
            kind: TrackKind::Audio,
        }))
        .map_err(|e| e.to_string())?;
    Ok(state.snapshot())
}

#[tauri::command]
pub async fn import_asset(state: State<'_, AppState>, uri: String) -> CmdResult<AssetView> {
    let local_path = uri.strip_prefix("file://").unwrap_or(&uri).to_string();
    let probe = slop_media::probe_asset(&local_path)
        .await
        .map_err(|e| e.to_string())?;
    let asset_id = ids::asset();
    let asset = probe.into_asset(asset_id.clone(), uri.clone());

    state
        .apply_and_log(Op::new(OpKind::AddAsset(asset.clone())))
        .map_err(|e| e.to_string())?;

    let view = AssetView {
        asset_id: asset_id.clone(),
        uri,
        duration_sec: asset.duration_sec,
        has_video: asset.has_video,
        has_audio: asset.has_audio,
        fps: asset.fps,
        resolution: asset.resolution,
        proxy_path: None,
        thumb_strip_path: None,
        transcript_status: JobStatus::Missing,
        scenes_status: JobStatus::Missing,
    };
    state.with(|s| {
        s.assets.insert(
            asset_id,
            AssetCache {
                view: view.clone(),
                transcript: None,
                scenes: Vec::new(),
            },
        );
    });
    Ok(view)
}

#[tauri::command]
pub async fn generate_proxies(state: State<'_, AppState>, asset_id: String) -> CmdResult<()> {
    let (uri, project_root) = state.with(|s| {
        let uri = s
            .assets
            .get(&asset_id)
            .map(|a| a.view.uri.clone())
            .unwrap_or_default();
        (uri, s.path.clone())
    });
    if uri.is_empty() {
        return Err("unknown asset_id".into());
    }
    let local = uri.strip_prefix("file://").unwrap_or(&uri).to_string();
    let proxy_dir = project_root.join("proxies");
    let proxy_path = proxy_dir.join(format!("{}.mp4", &asset_id));
    let thumb_path = proxy_dir.join(format!("{}.thumbs.png", &asset_id));

    state.with(|s| s.pending_jobs += 1);
    let opts = slop_media::ProxyOptions::default();
    let r = slop_media::generate_proxy(&local, &proxy_path, &opts).await;
    let probe = slop_media::probe_asset(&local).await.ok();
    if r.is_ok() {
        if let Some(p) = probe {
            let _ = slop_media::generate_thumb_strip(
                &local,
                p.duration_sec,
                &thumb_path,
                &slop_media::ThumbOptions::default(),
            )
            .await;
        }
    }
    state.with(|s| {
        s.pending_jobs = s.pending_jobs.saturating_sub(1);
        if let Some(a) = s.assets.get_mut(&asset_id) {
            a.view.proxy_path = Some(proxy_path);
            a.view.thumb_strip_path = Some(thumb_path);
        }
    });
    Ok(())
}

#[tauri::command]
pub async fn transcribe_asset(state: State<'_, AppState>, asset_id: String) -> CmdResult<()> {
    let (uri, duration_sec) = state.with(|s| {
        s.assets
            .get(&asset_id)
            .map(|a| (a.view.uri.clone(), a.view.duration_sec))
            .unwrap_or_default()
    });
    let local = uri.strip_prefix("file://").unwrap_or(&uri).to_string();
    state.with(|s| {
        if let Some(a) = s.assets.get_mut(&asset_id) {
            a.view.transcript_status = JobStatus::Running;
        }
        s.pending_jobs += 1;
    });

    let backend = slop_asr::backend::placeholder::PlaceholderBackend;
    let job = slop_asr::AsrJob {
        asset_id: asset_id.clone(),
        input: PathBuf::from(local),
        duration_sec,
    };
    let opts = slop_asr::AsrOptions::default();
    let result =
        <slop_asr::backend::placeholder::PlaceholderBackend as slop_asr::AsrBackend>::transcribe(
            &backend, job, &opts,
        )
        .await;

    state.with(|s| {
        s.pending_jobs = s.pending_jobs.saturating_sub(1);
        if let Some(a) = s.assets.get_mut(&asset_id) {
            match result {
                Ok(t) => {
                    a.transcript = Some(t);
                    a.view.transcript_status = JobStatus::Ready;
                }
                Err(_) => {
                    a.view.transcript_status = JobStatus::Error;
                }
            }
        }
    });
    Ok(())
}

#[tauri::command]
pub async fn detect_scenes(state: State<'_, AppState>, asset_id: String) -> CmdResult<()> {
    let (uri, duration_sec) = state.with(|s| {
        s.assets
            .get(&asset_id)
            .map(|a| (a.view.uri.clone(), a.view.duration_sec))
            .unwrap_or_default()
    });
    let local = uri.strip_prefix("file://").unwrap_or(&uri).to_string();
    state.with(|s| {
        if let Some(a) = s.assets.get_mut(&asset_id) {
            a.view.scenes_status = JobStatus::Running;
        }
        s.pending_jobs += 1;
    });
    let det = slop_scenes::ContentDetector::default();
    let scenes = slop_scenes::detect_scenes(&local, duration_sec, &det).await;
    state.with(|s| {
        s.pending_jobs = s.pending_jobs.saturating_sub(1);
        if let Some(a) = s.assets.get_mut(&asset_id) {
            match scenes {
                Ok(v) => {
                    a.scenes = v;
                    a.view.scenes_status = JobStatus::Ready;
                }
                Err(_) => {
                    a.view.scenes_status = JobStatus::Error;
                }
            }
        }
    });
    Ok(())
}

#[tauri::command]
pub async fn build_candidates(state: State<'_, AppState>) -> CmdResult<()> {
    let weights = ScoreWeights::default();
    state.with(|s| {
        let mut cands = Vec::new();
        for asset in s.assets.values() {
            if let Some(t) = &asset.transcript {
                let mut moms = score_moments(t, &asset.scenes, &weights);
                cands.append(&mut moms);
            }
        }
        s.candidates = cands;
    });
    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlannerStatusOut {
    pub ok: bool,
    pub message: String,
    pub repair_notes: Vec<String>,
}

#[tauri::command]
pub async fn plan_rough_cut(
    state: State<'_, AppState>,
    prompt: String,
) -> CmdResult<PlannerStatusOut> {
    let (pack, tl_clone, cfg) = state.with(|s| {
        let pack = build_prompt_pack(prompt, &s.timeline, vec![s.candidates.clone()], 120);
        (pack, s.timeline.clone(), s.endpoint.clone())
    });

    let result =
        slop_planner::plan(&cfg, &pack, &tl_clone, slop_planner::PromptStyle::default()).await;

    match result {
        Ok(r) => {
            let track_id = tl_clone
                .tracks
                .iter()
                .find(|t| matches!(t.kind, slop_core::TrackKind::Video))
                .map(|t| t.track_id.clone())
                .unwrap_or_default();
            let mut clips = Vec::new();
            for ptrack in r.plan.timeline.tracks.iter() {
                for pc in ptrack.clips.iter() {
                    let dur = pc.src_out - pc.src_in;
                    let item = slop_core::ClipItem {
                        item_id: ids::clip(),
                        asset_id: pc.asset_id.clone(),
                        src_in: pc.src_in,
                        src_out: pc.src_out,
                        timeline_in: pc.timeline_in,
                        timeline_out: pc.timeline_in + dur,
                        speed: 1.0,
                        effects: vec![],
                        markers: vec![],
                        metadata: slop_core::ClipMetadata {
                            selection_reason: Some(pc.reason.clone()),
                            score: None,
                            locked_by_user: false,
                            prompt_id: None,
                        },
                    };
                    clips.push(item);
                }
            }
            let captions: Vec<slop_core::Caption> = r
                .plan
                .captions
                .into_iter()
                .map(|c| slop_core::Caption {
                    timeline_in: c.timeline_in,
                    timeline_out: c.timeline_out,
                    text: c.text,
                    segment_id: c.segment_id,
                })
                .collect();

            // Replace the entire video track 0..end with the new clips.
            let last_end = clips.iter().map(|c| c.timeline_out).fold(0.0_f64, f64::max);
            state
                .apply_and_log(Op::new(OpKind::ReplaceTimelineRange {
                    track_id,
                    timeline_in: 0.0,
                    timeline_out: last_end.max(tl_clone.duration_sec()),
                    new_items: clips,
                    new_captions: captions,
                }))
                .map_err(|e| e.to_string())?;
            Ok(PlannerStatusOut {
                ok: true,
                message: r.plan.summary,
                repair_notes: r.repair_notes,
            })
        }
        Err(e) => Ok(PlannerStatusOut {
            ok: false,
            message: e.to_string(),
            repair_notes: vec![],
        }),
    }
}

#[tauri::command]
pub async fn regenerate_range(
    state: State<'_, AppState>,
    track_id: String,
    timeline_in: f64,
    timeline_out: f64,
    prompt: String,
) -> CmdResult<PlannerStatusOut> {
    let _ = (track_id, timeline_in, timeline_out);
    // V1: regenerate the whole timeline conditioned on the prompt; pinned
    // clips are protected by `ReplaceTimelineRange`. A future revision will
    // restrict the planner to candidates inside the requested range only.
    plan_rough_cut(state, prompt).await
}

#[tauri::command]
pub async fn pin_clip(
    state: State<'_, AppState>,
    track_id: String,
    item_id: String,
) -> CmdResult<()> {
    state
        .apply_and_log(Op::new(OpKind::PinClip { track_id, item_id }))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn unpin_clip(
    state: State<'_, AppState>,
    track_id: String,
    item_id: String,
) -> CmdResult<()> {
    state
        .apply_and_log(Op::new(OpKind::UnpinClip { track_id, item_id }))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn render_preview(state: State<'_, AppState>) -> CmdResult<String> {
    let (tl, root) = state.with(|s| (s.timeline.clone(), s.path.clone()));
    let out = root.join("preview.mp4");
    slop_render::render(&tl, &out, &slop_render::RenderOptions::default())
        .await
        .map_err(|e| e.to_string())?;
    Ok(out.to_string_lossy().to_string())
}

#[tauri::command]
pub async fn export_otio(state: State<'_, AppState>, out_path: String) -> CmdResult<()> {
    let tl = state.with(|s| s.timeline.clone());
    let p = PathBuf::from(out_path);
    slop_otio::write_otio(&tl, &p).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn get_timeline(state: State<'_, AppState>) -> CmdResult<slop_core::Timeline> {
    Ok(state.with(|s| s.timeline.clone()))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EndpointConfigOut {
    pub base_url: String,
    pub model: String,
    pub api_key_set: bool,
    pub temperature: f32,
}

#[tauri::command]
pub async fn get_endpoint_config(state: State<'_, AppState>) -> CmdResult<EndpointConfigOut> {
    Ok(state.with(|s| EndpointConfigOut {
        base_url: s.endpoint.base_url.clone(),
        model: s.endpoint.model.clone(),
        api_key_set: s.endpoint.api_key.as_deref().is_some_and(|k| !k.is_empty()),
        temperature: s.endpoint.temperature,
    }))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EndpointConfigIn {
    pub base_url: String,
    pub model: String,
    pub api_key: Option<String>,
    pub temperature: f32,
}

#[tauri::command]
pub async fn set_endpoint_config(
    state: State<'_, AppState>,
    cfg: EndpointConfigIn,
) -> CmdResult<()> {
    let url_local = is_local_url(&cfg.base_url);
    let privacy = state.with(|s| s.privacy_mode);
    if privacy && !url_local {
        return Err(format!(
            "privacy mode is enabled; only localhost endpoints are allowed (got {})",
            cfg.base_url
        ));
    }
    state.with(|s| {
        s.endpoint.base_url = cfg.base_url;
        s.endpoint.model = cfg.model;
        if let Some(k) = cfg.api_key {
            s.endpoint.api_key = if k.is_empty() { None } else { Some(k) };
        }
        s.endpoint.temperature = cfg.temperature;
    });
    Ok(())
}

/// Toggle privacy mode. Writing a `PRIVACY_MODE` sentinel into the project
/// root makes the choice survive crashes and acts as a future-proof gate
/// for additional networked features.
#[tauri::command]
pub async fn set_privacy_mode(state: State<'_, AppState>, on: bool) -> CmdResult<()> {
    let project_root = state.with(|s| s.path.clone());
    let endpoint_url = state.with(|s| s.endpoint.base_url.clone());
    if on && !is_local_url(&endpoint_url) {
        return Err(format!(
            "cannot enable privacy mode: current endpoint {} is not on localhost",
            endpoint_url
        ));
    }
    state.with(|s| s.privacy_mode = on);
    let sentinel = project_root.join("PRIVACY_MODE");
    if on {
        let _ = std::fs::write(&sentinel, "enabled\n");
    } else {
        let _ = std::fs::remove_file(&sentinel);
    }
    Ok(())
}

#[tauri::command]
pub async fn get_privacy_mode(state: State<'_, AppState>) -> CmdResult<bool> {
    Ok(state.with(|s| s.privacy_mode))
}

fn is_local_url(url: &str) -> bool {
    let lower = url.to_lowercase();
    lower.starts_with("http://localhost")
        || lower.starts_with("http://127.0.0.1")
        || lower.starts_with("http://[::1]")
        || lower.starts_with("https://localhost")
        || lower.starts_with("https://127.0.0.1")
        || lower.starts_with("https://[::1]")
}
