//! System + user message construction for the planner LLM.
//!
//! We follow the four-pass pattern from the design doc:
//!
//! 1. system message: act as a timeline compiler with explicit constraints,
//! 2. user message: project goal, candidate moments, allowed track ids,
//! 3. response_format: JSON Schema (`plan.v1.json`) constraint,
//! 4. (post-call) validate + repair + critic.
//!
//! This module owns step 1 and step 2.

use serde_json::json;
use slop_score::PromptPack;

/// Tone / instruction style for the system prompt.
#[derive(Debug, Clone, Copy, Default)]
pub enum PromptStyle {
    /// Conservative: prefer fewer clips, longer holds, easy cuts.
    #[default]
    RoughCut,
    /// Punchy: prefer many clips, short holds. For social shorts.
    Punchy,
    /// Quiet: prefer minimal cuts. For interview retention.
    Quiet,
}

/// Build the `messages` field for an OpenAI-compatible chat-completions
/// request, given a project goal, the prompt pack, and a style.
pub fn build_messages(pack: &PromptPack, style: PromptStyle) -> Vec<serde_json::Value> {
    let style_hint = match style {
        PromptStyle::RoughCut => {
            "Prefer fewer, longer clips that read clearly. Use cutaways only when they help \
             the story. Open with the strongest line you can find."
        }
        PromptStyle::Punchy => {
            "Prefer many short clips. Open with the strongest hook in the first 2 seconds. \
             Use cutaways aggressively to keep visual momentum."
        }
        PromptStyle::Quiet => {
            "Prefer long holds and minimal cuts. Cut only on strong content beats."
        }
    };

    let system = format!(
        "You are a rough-cut planning engine for an open-source video editor.\n\
         You may only select clips from the provided candidate list.\n\
         You must output valid JSON that conforms to the supplied schema.\n\
         You must never invent asset IDs, segment IDs, shot IDs, speakers, or timestamps.\n\
         If you cannot satisfy the goal, return an empty timeline and a clear `warnings` entry \
         explaining why.\n\
         Style: {style_hint}\n\
         Hard rules:\n\
         - All `asset_id`s must be drawn from the `assets` field.\n\
         - All `src_in`/`src_out` pairs must lie inside an asset's duration.\n\
         - `src_in` < `src_out` strictly.\n\
         - On a single track, clips must not overlap and `timeline_in` must be \
         non-decreasing.\n\
         - Track ids must be drawn from `allowed_track_ids`.\n\
         - Output the schema's required fields and only those."
    );

    let user_payload = json!({
        "goal": pack.goal,
        "fps": pack.fps,
        "assets": pack.assets,
        "allowed_track_ids": pack.allowed_track_ids,
        "candidates": pack.moments,
    });

    vec![
        json!({ "role": "system", "content": system }),
        json!({ "role": "user", "content": user_payload.to_string() }),
    ]
}
