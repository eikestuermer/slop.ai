//! `roughcut.v1` -> `roughcut.v2` migration.
//!
//! Lossless: every V1 document maps to a valid V2 document with new fields
//! filled in at their defaults. The reverse is *not* lossless: V2 documents
//! that use compound clips, multicam, transitions, effect graphs, styled
//! captions, or speed curves cannot be downgraded.

use serde_json::{json, Value};

/// Migrate a parsed V1 timeline JSON to V2 in place.
///
/// On error returns the offending JSON path. The migration is idempotent:
/// re-running on a V2 document is a no-op.
pub fn migrate_v1_to_v2(doc: &mut Value) -> Result<(), String> {
    let version = doc
        .get("schema_version")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "missing schema_version".to_string())?
        .to_string();
    if version == "roughcut.v2" {
        return Ok(());
    }
    if version != "roughcut.v1" {
        return Err(format!("unknown schema_version {version}"));
    }

    doc["schema_version"] = json!("roughcut.v2");

    // Add new project-level fields with defaults.
    if let Some(project) = doc.get_mut("project").and_then(|v| v.as_object_mut()) {
        project
            .entry("color_space".to_string())
            .or_insert(json!("rec709"));
        project
            .entry("audio_channels".to_string())
            .or_insert(json!(2));
    }

    // Mixer + color pipeline default scaffolds.
    doc["mixer"] = json!({
        "buses": [],
        "track_strips": [],
        "loudness_target": { "lufs": -14, "true_peak_dbfs": -1, "lra": 11 }
    });
    doc["color"] = json!({
        "working_space": "rec709",
        "output_transform": "rec709_2_4",
        "global_grade": {
            "lift": [0, 0, 0, 0],
            "gamma": [1, 1, 1, 1],
            "gain": [1, 1, 1, 1],
            "saturation": 1,
            "contrast": 1,
            "lut_uri": null,
            "wb_temperature": 6500,
            "wb_tint": 0
        }
    });

    if let Some(tracks) = doc.get_mut("tracks").and_then(|v| v.as_array_mut()) {
        for track in tracks.iter_mut() {
            if let Some(track_obj) = track.as_object_mut() {
                track_obj.entry("muted".to_string()).or_insert(json!(false));
                track_obj
                    .entry("locked".to_string())
                    .or_insert(json!(false));
            }
            if let Some(items) = track.get_mut("items").and_then(|v| v.as_array_mut()) {
                for item in items.iter_mut() {
                    let item_type = item
                        .get("type")
                        .and_then(|v| v.as_str())
                        .unwrap_or("clip")
                        .to_string();
                    if item_type == "clip" {
                        // Convert V1 scalar `speed` into a SpeedCurve number (still scalar in V2 oneOf).
                        // Effects: V1 had a small enum; V2 wants EffectNode {node_id, kind, ...}.
                        if let Some(effects) =
                            item.get_mut("effects").and_then(|v| v.as_array_mut())
                        {
                            for eff in effects.iter_mut() {
                                if eff.get("node_id").is_none() {
                                    let kind = eff
                                        .get("kind")
                                        .and_then(|v| v.as_str())
                                        .unwrap_or("fade_in")
                                        .to_string();
                                    let dur = eff.get("duration_sec").cloned();
                                    let mut params =
                                        eff.get("params").cloned().unwrap_or(json!({}));
                                    if let Some(d) = dur {
                                        if let Some(obj) = params.as_object_mut() {
                                            obj.insert("duration_sec".to_string(), d);
                                        }
                                    }
                                    *eff = json!({
                                        "node_id": format!("eff_{}", uuid::Uuid::new_v4().simple().to_string()[..8].to_string()),
                                        "kind": kind,
                                        "params": params,
                                        "keyframes": [],
                                        "bypass": false
                                    });
                                }
                            }
                        }
                        if let Some(item_obj) = item.as_object_mut() {
                            item_obj
                                .entry("audio_offset_sec".to_string())
                                .or_insert(json!(0));
                            item_obj
                                .entry("video_offset_sec".to_string())
                                .or_insert(json!(0));
                        }
                    }
                }
            }
        }
    }

    if let Some(captions) = doc.get_mut("captions").and_then(|v| v.as_array_mut()) {
        for cap in captions.iter_mut() {
            if let Some(obj) = cap.as_object_mut() {
                obj.entry("language".to_string()).or_insert(json!("en"));
                obj.entry("speaker".to_string()).or_insert(json!(null));
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migrates_minimal_v1_to_v2() {
        let mut doc = json!({
            "schema_version": "roughcut.v1",
            "project": { "fps": 30, "resolution": {"w": 1920, "h": 1080}, "sample_rate": 48000 },
            "assets": [],
            "tracks": [],
            "captions": []
        });
        migrate_v1_to_v2(&mut doc).unwrap();
        assert_eq!(doc["schema_version"], "roughcut.v2");
        assert!(doc["mixer"].is_object());
        assert!(doc["color"].is_object());
        assert_eq!(doc["project"]["color_space"], "rec709");
    }

    #[test]
    fn idempotent_on_v2() {
        let mut doc = json!({
            "schema_version": "roughcut.v2",
            "project": { "fps": 30, "resolution": {"w": 1920, "h": 1080}, "sample_rate": 48000 },
            "assets": [],
            "tracks": []
        });
        let before = doc.clone();
        migrate_v1_to_v2(&mut doc).unwrap();
        assert_eq!(before, doc);
    }

    #[test]
    fn rejects_unknown_version() {
        let mut doc =
            json!({"schema_version": "roughcut.v0", "project": {}, "assets": [], "tracks": []});
        assert!(migrate_v1_to_v2(&mut doc).is_err());
    }

    #[test]
    fn fade_in_effect_lands_in_v2_effect_graph() {
        let mut doc = json!({
            "schema_version": "roughcut.v1",
            "project": { "fps": 30, "resolution": {"w": 1920, "h": 1080}, "sample_rate": 48000 },
            "assets": [],
            "tracks": [{
                "track_id": "v1",
                "kind": "video",
                "items": [{
                    "type": "clip",
                    "item_id": "c1",
                    "asset_id": "a1",
                    "src_in": 0.0,
                    "src_out": 5.0,
                    "timeline_in": 0.0,
                    "timeline_out": 5.0,
                    "speed": 1.0,
                    "effects": [{ "kind": "fade_in", "duration_sec": 0.5 }],
                    "markers": [],
                    "metadata": {}
                }]
            }]
        });
        migrate_v1_to_v2(&mut doc).unwrap();
        let eff = &doc["tracks"][0]["items"][0]["effects"][0];
        assert!(eff["node_id"].as_str().unwrap().starts_with("eff_"));
        assert_eq!(eff["kind"], "fade_in");
        assert_eq!(eff["bypass"], false);
        assert!(eff["keyframes"].is_array());
        assert_eq!(eff["params"]["duration_sec"], 0.5);
    }

    #[test]
    fn track_muted_locked_defaults_added() {
        let mut doc = json!({
            "schema_version": "roughcut.v1",
            "project": { "fps": 30, "resolution": {"w": 1920, "h": 1080}, "sample_rate": 48000 },
            "assets": [],
            "tracks": [{
                "track_id": "v1",
                "kind": "video",
                "items": []
            }]
        });
        migrate_v1_to_v2(&mut doc).unwrap();
        assert_eq!(doc["tracks"][0]["muted"], false);
        assert_eq!(doc["tracks"][0]["locked"], false);
    }

    #[test]
    fn clip_gets_audio_video_offset_zero() {
        let mut doc = json!({
            "schema_version": "roughcut.v1",
            "project": { "fps": 30, "resolution": {"w": 1920, "h": 1080}, "sample_rate": 48000 },
            "assets": [],
            "tracks": [{
                "track_id": "v1",
                "kind": "video",
                "items": [{
                    "type": "clip",
                    "item_id": "c1",
                    "asset_id": "a1",
                    "src_in": 0.0, "src_out": 5.0,
                    "timeline_in": 0.0, "timeline_out": 5.0,
                    "speed": 1.0, "effects": [], "markers": [], "metadata": {}
                }]
            }]
        });
        migrate_v1_to_v2(&mut doc).unwrap();
        assert_eq!(doc["tracks"][0]["items"][0]["audio_offset_sec"], 0);
        assert_eq!(doc["tracks"][0]["items"][0]["video_offset_sec"], 0);
    }

    #[test]
    fn captions_get_language_default() {
        let mut doc = json!({
            "schema_version": "roughcut.v1",
            "project": { "fps": 30, "resolution": {"w": 1920, "h": 1080}, "sample_rate": 48000 },
            "assets": [],
            "tracks": [],
            "captions": [
                { "timeline_in": 0.0, "timeline_out": 1.0, "text": "hi" }
            ]
        });
        migrate_v1_to_v2(&mut doc).unwrap();
        assert_eq!(doc["captions"][0]["language"], "en");
        assert!(doc["captions"][0]["speaker"].is_null());
    }
}
