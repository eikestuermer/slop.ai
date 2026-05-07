//! Token budgeting for multi-modal planner requests.

use serde::{Deserialize, Serialize};

/// Visual budget for a single planner request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VisualBudget {
    /// Maximum number of frames in this request.
    pub max_frames: usize,
    /// Frames per candidate (1, 3, or 5 typically).
    pub frames_per_clip: usize,
    /// Tile resolution per side (square).
    pub tile_size: u32,
    /// Approximate input tokens per tile.
    pub tokens_per_tile: usize,
}

/// Plan a budget given a hard input-token cap and the number of candidates.
pub fn plan_visual_budget(max_input_tokens: usize, n_candidates: usize) -> VisualBudget {
    // Rough cost model from Qwen2.5-VL / Llava-OneVision (~1024 tokens per
    // 448-square tile after their patch projector).
    let tokens_per_tile = 1024usize;
    let max_frames = (max_input_tokens / tokens_per_tile).max(1);
    let frames_per_clip = if n_candidates == 0 {
        3
    } else if max_frames / n_candidates >= 5 {
        5
    } else if max_frames / n_candidates >= 3 {
        3
    } else if max_frames / n_candidates >= 1 {
        1
    } else {
        0
    };
    VisualBudget {
        max_frames,
        frames_per_clip,
        tile_size: 448,
        tokens_per_tile,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn budget_drops_to_zero_when_overcrowded() {
        let b = plan_visual_budget(2000, 100);
        assert_eq!(b.frames_per_clip, 0);
    }

    #[test]
    fn generous_budget_picks_5_frames() {
        let b = plan_visual_budget(50_000, 5);
        assert_eq!(b.frames_per_clip, 5);
    }

    #[test]
    fn tile_is_448_for_qwen_vl() {
        let b = plan_visual_budget(20_000, 5);
        assert_eq!(b.tile_size, 448);
    }
}
