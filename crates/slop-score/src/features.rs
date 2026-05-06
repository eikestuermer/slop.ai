//! Per-feature signal extraction.
//!
//! Each function in this module looks at a transcript / scene list / waveform
//! and returns a per-segment numeric feature in `[0, 1]`. The scorer combines
//! these via `ScoreWeights`.

use regex::Regex;
use slop_asr::Segment;
use slop_scenes::Scene;

/// Lexical highlight score in `[0, 1]`. Features:
///
/// - has a question mark: +0.4
/// - contains a number: +0.2
/// - contains a strong-affect word from a tiny built-in dictionary: +0.3
/// - all-caps or capitalized phrase (proxy for named entities): +0.1
pub fn lexical_score(segment: &Segment) -> f32 {
    let text = segment.text.trim();
    if text.is_empty() {
        return 0.0;
    }
    let mut score = 0.0_f32;

    if text.contains('?') {
        score += 0.4;
    }
    let number = Regex::new(r"\b\d+(\.\d+)?\b").unwrap();
    if number.is_match(text) {
        score += 0.2;
    }
    let strong = [
        "love", "hate", "incredible", "shocked", "first", "best", "worst",
        "biggest", "fastest", "secret", "truth", "actually", "really",
        "honestly", "huge", "amazing", "important",
    ];
    let lower = text.to_lowercase();
    if strong.iter().any(|w| lower.split_whitespace().any(|t| t.trim_matches(|c: char| !c.is_alphanumeric()) == *w)) {
        score += 0.3;
    }
    let capitalized = Regex::new(r"\b[A-Z][a-z]{2,}\b").unwrap();
    if capitalized.find_iter(text).count() >= 1 {
        score += 0.1;
    }

    score.min(1.0)
}

/// Speaker-turn score: a segment that starts a new speaker block is
/// assigned 1.0 (a "fresh take"); subsequent segments by the same speaker
/// fade over time.
pub fn speaker_turn_scores(segments: &[Segment]) -> Vec<f32> {
    let mut out = vec![0.0; segments.len()];
    let mut last_speaker: Option<&str> = None;
    let mut run_len = 0_u32;
    for (i, seg) in segments.iter().enumerate() {
        let sp = seg.speaker.as_deref();
        if sp != last_speaker {
            out[i] = 1.0;
            run_len = 1;
        } else {
            run_len += 1;
            out[i] = (1.0 / run_len as f32).max(0.1);
        }
        last_speaker = sp;
    }
    out
}

/// Scene-aligned score: a segment that lines up with a scene change scores
/// high (good cut moment); one that crosses a scene change scores low
/// (would create a jarring mid-shot cut).
pub fn scene_alignment_score(segment: &Segment, scenes: &[Scene]) -> f32 {
    if scenes.is_empty() {
        return 0.0;
    }
    let s = segment.start_sec;
    let e = segment.end_sec;
    let mut crosses = 0;
    let mut starts_aligned = false;
    let mut ends_aligned = false;
    for scene in scenes {
        if scene.start_sec > s + 0.01 && scene.start_sec < e - 0.01 {
            crosses += 1;
        }
        if (scene.start_sec - s).abs() < 0.5 {
            starts_aligned = true;
        }
        if (scene.end_sec - e).abs() < 0.5 {
            ends_aligned = true;
        }
    }
    let mut score = 0.0_f32;
    if starts_aligned {
        score += 0.5;
    }
    if ends_aligned {
        score += 0.3;
    }
    score -= 0.2 * crosses as f32;
    score.clamp(0.0, 1.0)
}

/// Duration-fit score: prefer 1.5..=8s segments (interview-friendly), and
/// penalize segments shorter than 0.5s or longer than 20s.
pub fn duration_score(segment: &Segment) -> f32 {
    let dur = (segment.end_sec - segment.start_sec).max(0.0);
    if dur < 0.5 {
        0.0
    } else if (1.5..=8.0).contains(&dur) {
        1.0
    } else if dur < 1.5 {
        dur / 1.5
    } else if dur <= 20.0 {
        ((20.0 - dur) / 12.0) as f32
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seg(start: f64, end: f64, text: &str, speaker: Option<&str>) -> Segment {
        Segment {
            segment_id: format!("seg_{start}"),
            start_sec: start,
            end_sec: end,
            speaker: speaker.map(String::from),
            text: text.into(),
            confidence: None,
        }
    }

    #[test]
    fn lexical_picks_up_question_and_number() {
        let s = seg(0.0, 5.0, "Did sales really grow 47%?", None);
        let v = lexical_score(&s);
        assert!(v > 0.5, "got {v}");
    }

    #[test]
    fn speaker_turn_first_segment_is_high() {
        let segs = vec![
            seg(0.0, 5.0, "hi", Some("S1")),
            seg(5.0, 10.0, "next", Some("S1")),
            seg(10.0, 15.0, "switch", Some("S2")),
        ];
        let v = speaker_turn_scores(&segs);
        assert!((v[0] - 1.0).abs() < 1e-6);
        assert!(v[1] < 1.0);
        assert!((v[2] - 1.0).abs() < 1e-6);
    }

    #[test]
    fn duration_score_peaks_in_sweet_spot() {
        let too_short = seg(0.0, 0.2, "x", None);
        let good = seg(0.0, 4.0, "x", None);
        let too_long = seg(0.0, 30.0, "x", None);
        assert!(duration_score(&too_short) < 0.1);
        assert!((duration_score(&good) - 1.0).abs() < 1e-6);
        assert!(duration_score(&too_long) < 0.1);
    }
}
