//! The "prompt pack": the structured payload that goes into the planner LLM
//! request. By construction it contains *only* the fields the model needs to
//! pick clips, never raw URIs or full transcripts.

use crate::moment::Moment;
use serde::{Deserialize, Serialize};
use slop_core::{Asset, Timeline};

/// Top-N moments per asset, plus light asset metadata, plus a project goal.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptPack {
    /// Free-text goal from the user.
    pub goal: String,
    /// Project frame rate.
    pub fps: f64,
    /// Asset-level metadata that the planner needs.
    pub assets: Vec<PromptAsset>,
    /// Track ids the planner is allowed to write to.
    pub allowed_track_ids: Vec<String>,
    /// Candidate moments (already filtered + sorted).
    pub moments: Vec<Moment>,
}

/// Slimmed asset metadata for the prompt.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptAsset {
    /// Asset id.
    pub asset_id: String,
    /// Duration in seconds.
    pub duration_sec: f64,
    /// True if the asset has video.
    pub has_video: bool,
    /// True if the asset has audio.
    pub has_audio: bool,
}

impl From<&Asset> for PromptAsset {
    fn from(a: &Asset) -> Self {
        Self {
            asset_id: a.asset_id.clone(),
            duration_sec: a.duration_sec,
            has_video: a.has_video,
            has_audio: a.has_audio,
        }
    }
}

/// Build a prompt pack from a project goal, current timeline, and the per-asset
/// scored candidate lists. Filters to the top-`top_n` moments per asset.
pub fn build_prompt_pack(
    goal: impl Into<String>,
    tl: &Timeline,
    moments_by_asset: Vec<Vec<Moment>>,
    top_n: usize,
) -> PromptPack {
    let mut moments: Vec<Moment> = moments_by_asset
        .into_iter()
        .flat_map(|mut v| {
            v.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
            v.truncate(top_n);
            v
        })
        .collect();

    moments.sort_by(|a, b| {
        a.asset_id
            .cmp(&b.asset_id)
            .then(a.start_sec.partial_cmp(&b.start_sec).unwrap())
    });

    PromptPack {
        goal: goal.into(),
        fps: tl.project.fps,
        assets: tl.assets.iter().map(PromptAsset::from).collect(),
        allowed_track_ids: tl.tracks.iter().map(|t| t.track_id.clone()).collect(),
        moments,
    }
}
