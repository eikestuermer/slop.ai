//! Slop AI CLI.
//!
//! ```text
//! slop ingest <project-dir> <media>...     Probe + proxy + transcript + scenes.
//! slop plan <project-dir> --prompt "..."   Generate a rough cut.
//! slop render <project-dir> --out cut.mp4  Compile to MP4.
//! slop export <project-dir> --target otio  Export OTIO / FCP7 / FCPXML / Kdenlive.
//! ```

use clap::{Parser, Subcommand, ValueEnum};
use slop_core::{ids, Op, OpKind, Timeline, TrackKind};
use slop_planner::{plan as run_plan, EndpointConfig, PromptStyle};
use slop_render::{render, RenderOptions};
use std::path::{Path, PathBuf};

#[derive(Parser)]
#[command(name = "slop", version, about)]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Probe + proxy + transcript + scenes for one or more media files.
    Ingest(IngestArgs),
    /// Run the planner against a project's transcripts and scenes.
    Plan(PlanArgs),
    /// Render the project's current timeline to MP4.
    Render(RenderArgs),
    /// Export to OTIO or a pro-NLE format.
    Export(ExportArgs),
}

#[derive(Parser)]
struct IngestArgs {
    /// Project directory.
    project: PathBuf,
    /// Media files to ingest.
    media: Vec<PathBuf>,
}

#[derive(Parser)]
struct PlanArgs {
    /// Project directory.
    project: PathBuf,
    /// Goal prompt.
    #[arg(long)]
    prompt: String,
    /// Endpoint URL.
    #[arg(long, default_value = "http://localhost:11434/v1")]
    endpoint: String,
    /// Model name.
    #[arg(long, default_value = "qwen3:8b")]
    model: String,
    /// Style.
    #[arg(long, value_enum, default_value_t = PromptStyleArg::RoughCut)]
    style: PromptStyleArg,
}

#[derive(Parser)]
struct RenderArgs {
    /// Project directory.
    project: PathBuf,
    /// Output MP4 path.
    #[arg(long)]
    out: PathBuf,
}

#[derive(Parser)]
struct ExportArgs {
    /// Project directory.
    project: PathBuf,
    /// Target.
    #[arg(long, value_enum)]
    target: ExportTarget,
    /// Output path.
    #[arg(long)]
    out: PathBuf,
}

#[derive(Clone, Copy, ValueEnum)]
enum ExportTarget {
    Otio,
    Fcp7,
    Fcpxml,
    Kdenlive,
}

#[derive(Clone, Copy, ValueEnum)]
enum PromptStyleArg {
    RoughCut,
    Punchy,
    Quiet,
}

impl From<PromptStyleArg> for PromptStyle {
    fn from(p: PromptStyleArg) -> PromptStyle {
        match p {
            PromptStyleArg::RoughCut => PromptStyle::RoughCut,
            PromptStyleArg::Punchy => PromptStyle::Punchy,
            PromptStyleArg::Quiet => PromptStyle::Quiet,
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let cli = Cli::parse();
    match cli.cmd {
        Cmd::Ingest(a) => cmd_ingest(a).await,
        Cmd::Plan(a) => cmd_plan(a).await,
        Cmd::Render(a) => cmd_render(a).await,
        Cmd::Export(a) => cmd_export(a).await,
    }
}

async fn cmd_ingest(a: IngestArgs) -> anyhow::Result<()> {
    let project = a.project;
    std::fs::create_dir_all(&project)?;
    let mut tl = load_or_init(&project)?;

    for path in a.media {
        let probe = slop_media::probe_asset(&path).await?;
        let asset_id = ids::asset();
        let uri = format!("file://{}", path.to_string_lossy());
        let asset = probe.into_asset(asset_id.clone(), uri);
        let op = Op::new(OpKind::AddAsset(asset));
        slop_core::reducer::apply(&mut tl, &op).map_err(|e| anyhow::anyhow!("{e}"))?;
        slop_core::OpLog::append_to_file(&op, project.join("ops.jsonl"))?;
        tracing::info!(asset_id = %asset_id, "asset added");

        let proxies_dir = project.join("proxies");
        slop_media::generate_proxy(
            &path,
            proxies_dir.join(format!("{asset_id}.mp4")),
            &slop_media::ProxyOptions::default(),
        )
        .await?;

        let det = slop_scenes::ContentDetector::default();
        let scenes =
            slop_scenes::detect_scenes(&path, tl.asset(&asset_id).unwrap().duration_sec, &det)
                .await?;
        let scenes_path = project.join("scenes").join(format!("{asset_id}.json"));
        std::fs::create_dir_all(scenes_path.parent().unwrap())?;
        std::fs::write(&scenes_path, serde_json::to_string_pretty(&scenes)?)?;
        tracing::info!(asset_id = %asset_id, n_scenes = scenes.len(), "scenes detected");
    }
    Ok(())
}

async fn cmd_plan(a: PlanArgs) -> anyhow::Result<()> {
    let mut tl = load_or_init(&a.project)?;
    if tl.tracks.is_empty() {
        let v = ids::track();
        let op = Op::new(OpKind::AddTrack {
            track_id: v.clone(),
            kind: TrackKind::Video,
        });
        slop_core::reducer::apply(&mut tl, &op).map_err(|e| anyhow::anyhow!("{e}"))?;
        slop_core::OpLog::append_to_file(&op, a.project.join("ops.jsonl"))?;
    }

    // Build a candidate set from any cached transcripts + scenes.
    let mut candidates = Vec::new();
    for asset in &tl.assets {
        let scenes_path = a
            .project
            .join("scenes")
            .join(format!("{}.json", asset.asset_id));
        let scenes: Vec<slop_scenes::Scene> = if scenes_path.is_file() {
            serde_json::from_str(&std::fs::read_to_string(&scenes_path)?)?
        } else {
            Vec::new()
        };
        // No transcript on the CLI in V3.0; the planner can still pick on
        // scene boundaries alone.
        let synthetic = slop_asr::Transcript {
            asset_id: asset.asset_id.clone(),
            backend: "synthetic".into(),
            model: "scene-only".into(),
            language: Some("auto".into()),
            segments: scenes
                .iter()
                .map(|s| slop_asr::Segment {
                    segment_id: s.scene_id.clone(),
                    start_sec: s.start_sec,
                    end_sec: s.end_sec,
                    speaker: None,
                    text: String::new(),
                    confidence: None,
                })
                .collect(),
        };
        let scored =
            slop_score::score_moments(&synthetic, &scenes, &slop_score::ScoreWeights::default());
        candidates.extend(scored);
    }
    let pack = slop_score::build_prompt_pack(a.prompt.clone(), &tl, vec![candidates], 200);
    let cfg = EndpointConfig {
        base_url: a.endpoint,
        model: a.model,
        ..Default::default()
    };
    let result = run_plan(&cfg, &pack, &tl, a.style.into()).await?;
    println!("{}", serde_json::to_string_pretty(&result.plan)?);
    Ok(())
}

async fn cmd_render(a: RenderArgs) -> anyhow::Result<()> {
    let tl = load_or_init(&a.project)?;
    render(&tl, &a.out, &RenderOptions::default()).await?;
    println!("rendered to {}", a.out.display());
    Ok(())
}

async fn cmd_export(a: ExportArgs) -> anyhow::Result<()> {
    let tl = load_or_init(&a.project)?;
    match a.target {
        ExportTarget::Otio => slop_otio::write_otio(&tl, &a.out)?,
        ExportTarget::Fcp7 => slop_otio::write_fcp7_xml(&tl, &a.out)?,
        ExportTarget::Fcpxml => slop_otio::write_resolve_fcpxml(&tl, &a.out)?,
        ExportTarget::Kdenlive => slop_otio::write_kdenlive_xml(&tl, &a.out)?,
    }
    println!("exported to {}", a.out.display());
    Ok(())
}

fn load_or_init(project: &Path) -> anyhow::Result<Timeline> {
    let ops_path = project.join("ops.jsonl");
    let mut tl = Timeline::empty();
    if ops_path.is_file() {
        let log = slop_core::OpLog::load(&ops_path).map_err(|e| anyhow::anyhow!("{e}"))?;
        tl = slop_core::reducer::replay(log.ops()).map_err(|e| anyhow::anyhow!("{e}"))?;
    }
    Ok(tl)
}
