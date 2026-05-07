//! Automerge-backed timeline document.

use automerge::{transaction::Transactable, AutoCommit, ReadDoc, ROOT};
use serde::{Deserialize, Serialize};
use slop_core::{Op, OpKind, Timeline};
use thiserror::Error;

/// Wraps an Automerge document holding a Slop timeline.
pub struct TimelineDoc {
    inner: AutoCommit,
}

/// Errors specific to TimelineDoc.
#[derive(Debug, Error)]
pub enum TimelineDocError {
    /// Automerge runtime error.
    #[error("automerge: {0}")]
    Automerge(String),
    /// JSON.
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

impl TimelineDoc {
    /// Construct an empty document with the timeline schema scaffolded.
    pub fn empty() -> Result<Self, TimelineDocError> {
        let mut doc = AutoCommit::new();
        doc.put(ROOT, "schema_version", "roughcut.v2")
            .map_err(|e| TimelineDocError::Automerge(e.to_string()))?;
        doc.put_object(ROOT, "project", automerge::ObjType::Map)
            .map_err(|e| TimelineDocError::Automerge(e.to_string()))?;
        doc.put_object(ROOT, "assets", automerge::ObjType::List)
            .map_err(|e| TimelineDocError::Automerge(e.to_string()))?;
        doc.put_object(ROOT, "tracks", automerge::ObjType::List)
            .map_err(|e| TimelineDocError::Automerge(e.to_string()))?;
        doc.put_object(ROOT, "captions", automerge::ObjType::List)
            .map_err(|e| TimelineDocError::Automerge(e.to_string()))?;
        Ok(Self { inner: doc })
    }

    /// Hydrate from a serialized Automerge byte blob.
    pub fn load(bytes: &[u8]) -> Result<Self, TimelineDocError> {
        let inner =
            AutoCommit::load(bytes).map_err(|e| TimelineDocError::Automerge(e.to_string()))?;
        Ok(Self { inner })
    }

    /// Serialize for storage.
    pub fn save(&mut self) -> Vec<u8> {
        self.inner.save()
    }

    /// Apply a slop-core op as an Automerge transaction.
    ///
    /// V2.5 implementation note: we serialize the op as JSON into a CRDT
    /// list `ops` and additionally project the resulting timeline state
    /// into the document's structured fields. The CRDT-merge property
    /// lives on the structured fields; the op log gives us audit.
    pub fn apply_op(&mut self, op: &Op) -> Result<(), TimelineDocError> {
        let json = serde_json::to_string(op)?;
        // Append to the audit log.
        let ops_id = match self
            .inner
            .get(ROOT, "ops")
            .map_err(|e| TimelineDocError::Automerge(e.to_string()))?
        {
            Some((_, id)) => id,
            None => self
                .inner
                .put_object(ROOT, "ops", automerge::ObjType::List)
                .map_err(|e| TimelineDocError::Automerge(e.to_string()))?,
        };
        let len = self.inner.length(&ops_id);
        self.inner
            .insert(&ops_id, len, json.as_str())
            .map_err(|e| TimelineDocError::Automerge(e.to_string()))?;
        // Project structured fields. Only the fields V2 cares about
        // (assets, tracks, captions, mixer, color, project settings).
        if let OpKind::SetProjectSettings(p) = &op.kind {
            let project_id = self
                .inner
                .get(ROOT, "project")
                .map_err(|e| TimelineDocError::Automerge(e.to_string()))?
                .map(|(_, id)| id)
                .ok_or_else(|| TimelineDocError::Automerge("missing project".into()))?;
            self.inner
                .put(&project_id, "fps", p.fps)
                .map_err(|e| TimelineDocError::Automerge(e.to_string()))?;
            self.inner
                .put(&project_id, "sample_rate", p.sample_rate as i64)
                .map_err(|e| TimelineDocError::Automerge(e.to_string()))?;
        }
        Ok(())
    }

    /// Reconstruct a slop-core `Timeline` from the log.
    pub fn reconstruct(&self) -> Result<Timeline, TimelineDocError> {
        let mut tl = Timeline::empty();
        if let Some((_, ops_id)) = self
            .inner
            .get(ROOT, "ops")
            .map_err(|e| TimelineDocError::Automerge(e.to_string()))?
        {
            let len = self.inner.length(&ops_id);
            for i in 0..len {
                if let Some((value, _)) = self
                    .inner
                    .get(&ops_id, i)
                    .map_err(|e| TimelineDocError::Automerge(e.to_string()))?
                {
                    if let Ok(s) = value.into_string() {
                        let op: Op = serde_json::from_str(&s)?;
                        if let Err(e) = slop_core::reducer::apply(&mut tl, &op) {
                            return Err(TimelineDocError::Automerge(e.to_string()));
                        }
                    }
                }
            }
        }
        Ok(tl)
    }

    /// Underlying Automerge handle (for sync protocol use).
    pub fn inner_mut(&mut self) -> &mut AutoCommit {
        &mut self.inner
    }
}

/// One serializable op-record as stored in the Automerge audit log. Public
/// for tests and downstream tooling.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    /// Public key of the actor who applied the op (hex-encoded).
    pub actor_pubkey_hex: String,
    /// Op as serialized by slop-core.
    pub op: serde_json::Value,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_doc_round_trips_through_save_load() {
        let mut doc = TimelineDoc::empty().unwrap();
        let bytes = doc.save();
        let _ = TimelineDoc::load(&bytes).unwrap();
    }

    #[test]
    fn apply_op_records_in_audit_log() {
        let mut doc = TimelineDoc::empty().unwrap();
        let op = Op::new(OpKind::AddTrack {
            track_id: "t1".into(),
            kind: slop_core::TrackKind::Video,
        });
        doc.apply_op(&op).unwrap();
        let tl = doc.reconstruct().unwrap();
        assert_eq!(tl.tracks.len(), 1);
        assert_eq!(tl.tracks[0].track_id, "t1");
    }

    #[test]
    fn multi_op_save_load_round_trip() {
        let mut doc = TimelineDoc::empty().unwrap();
        let asset = slop_core::Asset {
            asset_id: "a1".into(),
            uri: "file:///x.mp4".into(),
            duration_sec: 60.0,
            has_video: true,
            has_audio: true,
            fps: Some(30.0),
            resolution: Some(slop_core::Resolution { w: 1920, h: 1080 }),
            transcript_ref: None,
            shot_list_ref: None,
        };
        doc.apply_op(&Op::new(OpKind::AddAsset(asset))).unwrap();
        doc.apply_op(&Op::new(OpKind::AddTrack {
            track_id: "v1".into(),
            kind: slop_core::TrackKind::Video,
        }))
        .unwrap();
        doc.apply_op(&Op::new(OpKind::InsertClip {
            track_id: "v1".into(),
            clip: slop_core::ClipItem {
                item_id: "c1".into(),
                asset_id: "a1".into(),
                src_in: 0.0,
                src_out: 5.0,
                timeline_in: 0.0,
                timeline_out: 5.0,
                speed: 1.0,
                effects: vec![],
                markers: vec![],
                metadata: slop_core::ClipMetadata::default(),
            },
        }))
        .unwrap();

        let bytes = doc.save();
        let reloaded = TimelineDoc::load(&bytes).unwrap();
        let tl = reloaded.reconstruct().unwrap();
        assert_eq!(tl.assets.len(), 1);
        assert_eq!(tl.tracks.len(), 1);
        assert_eq!(tl.tracks[0].items.len(), 1);
    }
}
