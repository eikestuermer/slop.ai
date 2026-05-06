//! Combine per-feature signals into a final scored moment list.

use crate::features::*;
use crate::moment::{Moment, MomentBuilder};
use slop_asr::Transcript;
use slop_scenes::Scene;
use uuid::Uuid;

/// Per-feature weights used by [`score_moments`].
#[derive(Debug, Clone, Copy)]
pub struct ScoreWeights {
    /// Weight for lexical score.
    pub lexical: f32,
    /// Weight for speaker-turn score.
    pub speaker: f32,
    /// Weight for scene-alignment score.
    pub scene: f32,
    /// Weight for duration-fit score.
    pub duration: f32,
}

impl Default for ScoreWeights {
    fn default() -> Self {
        Self {
            lexical: 0.4,
            speaker: 0.2,
            scene: 0.25,
            duration: 0.15,
        }
    }
}

/// Build a scored candidate list from a transcript and the asset's scene
/// list.
pub fn score_moments(
    transcript: &Transcript,
    scenes: &[Scene],
    weights: &ScoreWeights,
) -> Vec<Moment> {
    let speaker_scores = speaker_turn_scores(&transcript.segments);
    let mut builder = MomentBuilder::new();

    for (i, seg) in transcript.segments.iter().enumerate() {
        let lex = lexical_score(seg);
        let spk = speaker_scores.get(i).copied().unwrap_or(0.0);
        let sc = scene_alignment_score(seg, scenes);
        let dur = duration_score(seg);

        let total = (weights.lexical * lex
            + weights.speaker * spk
            + weights.scene * sc
            + weights.duration * dur)
            .clamp(0.0, 1.0);

        let shot_id = scenes
            .iter()
            .find(|s| seg.start_sec >= s.start_sec - 1e-3 && seg.start_sec < s.end_sec + 1e-3)
            .map(|s| s.scene_id.clone());

        builder.push(Moment {
            moment_id: format!("m_{}", &Uuid::new_v4().simple().to_string()[..12]),
            asset_id: transcript.asset_id.clone(),
            start_sec: seg.start_sec,
            end_sec: seg.end_sec,
            segment_id: Some(seg.segment_id.clone()),
            shot_id,
            speaker: seg.speaker.clone(),
            text: seg.text.clone(),
            score: total,
            features: vec![
                ("lexical".into(), lex),
                ("speaker".into(), spk),
                ("scene".into(), sc),
                ("duration".into(), dur),
            ],
        });
    }

    builder.into_vec()
}

#[cfg(test)]
mod tests {
    use super::*;
    use slop_asr::Segment;

    fn seg(id: &str, s: f64, e: f64, text: &str, speaker: Option<&str>) -> Segment {
        Segment {
            segment_id: id.into(),
            start_sec: s,
            end_sec: e,
            speaker: speaker.map(String::from),
            text: text.into(),
            confidence: None,
        }
    }

    #[test]
    fn scores_in_zero_to_one() {
        let t = Transcript {
            asset_id: "a1".into(),
            backend: "placeholder".into(),
            model: "x".into(),
            language: Some("en".into()),
            segments: vec![
                seg("s1", 0.0, 4.0, "Did revenue grow 47%?", Some("S1")),
                seg("s2", 4.0, 12.0, "yes", Some("S1")),
                seg("s3", 12.0, 16.0, "incredible reveal here", Some("S2")),
            ],
        };
        let scenes = vec![Scene {
            scene_id: "shot_0000".into(),
            start_sec: 0.0,
            end_sec: 4.0,
        }];
        let moms = score_moments(&t, &scenes, &ScoreWeights::default());
        for m in &moms {
            assert!((0.0..=1.0).contains(&m.score), "{m:?}");
        }
        // The question segment with scene-alignment + lexical should be the
        // top scorer.
        let max = moms.iter().map(|m| m.score).fold(0.0_f32, f32::max);
        assert!((moms[0].score - max).abs() < 1e-6);
    }
}
