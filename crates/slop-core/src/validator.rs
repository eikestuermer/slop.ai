//! Validate `Timeline`s and `Plan`s against semantic invariants and against
//! the canonical JSON Schemas in `packages/schemas/`.
//!
//! ## Levels
//!
//! - [`validate_timeline_schema`] - draft-2020-12 schema validation against
//!   `timeline.v1.json`.
//! - [`validate_timeline_semantics`] - referential integrity, range
//!   correctness, no-overlap, etc.
//! - [`validate_plan_schema`] - schema validation against `plan.v1.json`.
//! - [`validate_plan_semantics`] - candidate-set discipline:
//!   - every `asset_id` exists,
//!   - every `src_in/src_out` is within the asset duration,
//!   - `src_in < src_out`,
//!   - clips do not overlap on the same track,
//!   - timeline times are monotone non-decreasing.
//!
//! Schema validation runs the schema text shipped with this crate (a copy
//! of `packages/schemas/*.json`); the build script is responsible for
//! keeping them synchronized.

use crate::{error::Result, plan::Plan, timeline::*, Error};

/// JSON Schema for `Timeline`. Embedded at compile time.
pub const TIMELINE_SCHEMA: &str =
    include_str!("../../../packages/schemas/timeline.v1.json");

/// JSON Schema for the LLM `Plan` contract. Embedded at compile time.
pub const PLAN_SCHEMA: &str = include_str!("../../../packages/schemas/plan.v1.json");

/// JSON Schema for the op log envelope.
pub const OPS_SCHEMA: &str = include_str!("../../../packages/schemas/ops.v1.json");

/// Validate a `Timeline` JSON document against `timeline.v1.json`.
pub fn validate_timeline_schema(value: &serde_json::Value) -> Result<()> {
    let schema: serde_json::Value = serde_json::from_str(TIMELINE_SCHEMA)?;
    let compiled = jsonschema::JSONSchema::options()
        .with_draft(jsonschema::Draft::Draft202012)
        .compile(&schema)
        .map_err(|e| Error::Schema(format!("compile timeline schema: {e}")))?;
    let result = compiled.validate(value);
    if let Err(errors) = result {
        let msg = errors
            .map(|e| format!("{}: {}", e.instance_path, e))
            .collect::<Vec<_>>()
            .join("; ");
        return Err(Error::Schema(msg));
    }
    Ok(())
}

/// Validate a parsed `Plan` (typed) value against `plan.v1.json`.
pub fn validate_plan_schema(value: &serde_json::Value) -> Result<()> {
    let schema: serde_json::Value = serde_json::from_str(PLAN_SCHEMA)?;
    let compiled = jsonschema::JSONSchema::options()
        .with_draft(jsonschema::Draft::Draft202012)
        .compile(&schema)
        .map_err(|e| Error::Schema(format!("compile plan schema: {e}")))?;
    let result = compiled.validate(value);
    if let Err(errors) = result {
        let msg = errors
            .map(|e| format!("{}: {}", e.instance_path, e))
            .collect::<Vec<_>>()
            .join("; ");
        return Err(Error::Schema(msg));
    }
    Ok(())
}

/// Validate semantic invariants on a `Timeline`.
pub fn validate_timeline_semantics(tl: &Timeline) -> Result<()> {
    for track in &tl.tracks {
        // Walk items in timeline order to check for overlaps.
        let mut ordered: Vec<&TrackItem> = track.items.iter().collect();
        ordered.sort_by(|a, b| a.timeline_in().partial_cmp(&b.timeline_in()).unwrap());

        for window in ordered.windows(2) {
            let (a, b) = (window[0], window[1]);
            if a.timeline_out() > b.timeline_in() + 1e-6 {
                return Err(Error::Overlap {
                    track: track.track_id.clone(),
                    a: a.id().to_string(),
                    b: b.id().to_string(),
                    a_in: a.timeline_in(),
                    a_out: a.timeline_out(),
                    b_in: b.timeline_in(),
                    b_out: b.timeline_out(),
                });
            }
        }

        for item in &track.items {
            if let TrackItem::Clip(c) = item {
                if !(c.src_in < c.src_out) {
                    return Err(Error::SrcEmpty(c.item_id.clone()));
                }
                let asset = tl
                    .asset(&c.asset_id)
                    .ok_or_else(|| Error::UnknownAsset(c.asset_id.clone()))?;
                if c.src_in < -1e-6 || c.src_out > asset.duration_sec + 1e-6 {
                    return Err(Error::SrcOutOfRange {
                        item_id: c.item_id.clone(),
                        src_in: c.src_in,
                        src_out: c.src_out,
                        duration: asset.duration_sec,
                    });
                }
            }
        }
    }
    Ok(())
}

/// Validate a `Plan` against the asset/candidate set it was generated for.
pub fn validate_plan_semantics(plan: &Plan, tl: &Timeline) -> Result<()> {
    let json = serde_json::to_value(plan)?;
    validate_plan_schema(&json)?;

    if plan.version != crate::PLAN_VERSION {
        return Err(Error::Schema(format!(
            "plan version {} != expected {}",
            plan.version,
            crate::PLAN_VERSION
        )));
    }

    for track in &plan.timeline.tracks {
        let mut last_out = 0.0_f64;
        for clip in &track.clips {
            let asset = tl
                .asset(&clip.asset_id)
                .ok_or_else(|| Error::UnknownAsset(clip.asset_id.clone()))?;
            if !(clip.src_in < clip.src_out) {
                return Err(Error::SrcEmpty(format!(
                    "{}@{}",
                    clip.asset_id, clip.timeline_in
                )));
            }
            if clip.src_in < -1e-6 || clip.src_out > asset.duration_sec + 1e-6 {
                return Err(Error::SrcOutOfRange {
                    item_id: format!("{}@{}", clip.asset_id, clip.timeline_in),
                    src_in: clip.src_in,
                    src_out: clip.src_out,
                    duration: asset.duration_sec,
                });
            }
            if clip.timeline_in + 1e-6 < last_out {
                return Err(Error::Overlap {
                    track: track.id.clone(),
                    a: "previous".into(),
                    b: format!("{}@{}", clip.asset_id, clip.timeline_in),
                    a_in: 0.0,
                    a_out: last_out,
                    b_in: clip.timeline_in,
                    b_out: clip.timeline_in + (clip.src_out - clip.src_in),
                });
            }
            last_out = clip.timeline_in + (clip.src_out - clip.src_in);
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids;

    fn empty_tl_json() -> serde_json::Value {
        serde_json::to_value(Timeline::empty()).unwrap()
    }

    #[test]
    fn schema_accepts_empty_timeline() {
        validate_timeline_schema(&empty_tl_json()).unwrap();
    }

    #[test]
    fn schema_rejects_bad_fps() {
        let mut v = empty_tl_json();
        v["project"]["fps"] = serde_json::json!(0);
        assert!(validate_timeline_schema(&v).is_err());
    }

    #[test]
    fn semantics_detect_overlap() {
        let mut tl = Timeline::empty();
        let aid = ids::asset();
        tl.assets.push(Asset {
            asset_id: aid.clone(),
            uri: "file:///x.mp4".into(),
            duration_sec: 60.0,
            has_video: true,
            has_audio: true,
            fps: None,
            resolution: None,
            transcript_ref: None,
            shot_list_ref: None,
        });
        let tid = ids::track();
        tl.tracks.push(Track {
            track_id: tid.clone(),
            kind: TrackKind::Video,
            items: vec![
                TrackItem::Clip(ClipItem {
                    item_id: "c1".into(),
                    asset_id: aid.clone(),
                    src_in: 0.0,
                    src_out: 5.0,
                    timeline_in: 0.0,
                    timeline_out: 5.0,
                    speed: 1.0,
                    effects: vec![],
                    markers: vec![],
                    metadata: ClipMetadata::default(),
                }),
                TrackItem::Clip(ClipItem {
                    item_id: "c2".into(),
                    asset_id: aid,
                    src_in: 0.0,
                    src_out: 5.0,
                    timeline_in: 4.0,
                    timeline_out: 9.0,
                    speed: 1.0,
                    effects: vec![],
                    markers: vec![],
                    metadata: ClipMetadata::default(),
                }),
            ],
        });
        let r = validate_timeline_semantics(&tl);
        assert!(matches!(r, Err(Error::Overlap { .. })));
    }
}
