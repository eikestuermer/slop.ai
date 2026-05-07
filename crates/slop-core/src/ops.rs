//! The reversible op log.
//!
//! Every mutation to a [`crate::Timeline`] is recorded as an [`Op`]. The op
//! log is a flat append-only file (`ops.jsonl`, one JSON object per line)
//! that:
//!
//! - is the source of truth for project state (the in-memory `Timeline` is a
//!   cache),
//! - supports undo/redo by computing inverses,
//! - supports crash recovery by replaying from an empty `Timeline`.
//!
//! Inverses are derived deterministically by the reducer when an op is
//! applied; we do not trust the inverse field stored on disk for safety
//! invariants.

use crate::{ids, timeline::*};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// A single recorded mutation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Op {
    /// Stable id, e.g. `op_xxxxxxxxxxxx`.
    pub op_id: String,
    /// Wall-clock timestamp.
    pub ts: DateTime<Utc>,
    /// Who issued this op.
    #[serde(default)]
    pub actor: Actor,
    /// Originating prompt id, if any.
    #[serde(default)]
    pub prompt_id: Option<String>,
    /// What kind of op.
    pub kind: OpKind,
}

impl Op {
    /// Construct a new op with a fresh id and current timestamp.
    pub fn new(kind: OpKind) -> Self {
        Self {
            op_id: ids::op(),
            ts: Utc::now(),
            actor: Actor::User,
            prompt_id: None,
            kind,
        }
    }

    /// Tag this op as being issued by the planner with the given prompt id.
    pub fn from_planner(self, prompt_id: impl Into<String>) -> Self {
        Self {
            actor: Actor::Planner,
            prompt_id: Some(prompt_id.into()),
            ..self
        }
    }
}

/// Who emitted an op.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Actor {
    /// A direct user action.
    #[default]
    User,
    /// The planner LLM.
    Planner,
    /// Internal system housekeeping.
    System,
}

/// All op kinds. Each variant carries a payload that is sufficient to apply
/// it to a `Timeline` and to compute its inverse.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", content = "payload")]
pub enum OpKind {
    /// Add a new asset.
    AddAsset(Asset),
    /// Remove an asset by id. Inverse needs to record the asset.
    RemoveAsset {
        /// Asset id to remove.
        asset_id: String,
    },
    /// Insert an empty track at the end of the track list.
    AddTrack {
        /// New track id.
        track_id: String,
        /// Track kind.
        kind: TrackKind,
    },
    /// Remove a track. Used as the inverse of `AddTrack`.
    RemoveTrack {
        /// Track id to remove.
        track_id: String,
    },
    /// Reinsert a previously removed track at the given index. Inverse-only.
    ReinsertTrack {
        /// The track to reinsert.
        track: Track,
        /// Original index in `tracks[]`.
        index: usize,
    },
    /// Insert a clip onto a track.
    InsertClip {
        /// Target track.
        track_id: String,
        /// The clip to insert.
        clip: ClipItem,
    },
    /// Remove a clip by item id.
    RemoveClip {
        /// Track id.
        track_id: String,
        /// Item id.
        item_id: String,
    },
    /// Reinsert a previously removed gap. Inverse-only.
    ReinsertGap {
        /// Track id.
        track_id: String,
        /// The gap to reinsert.
        gap: GapItem,
    },
    /// Change a clip's source range.
    TrimClip {
        /// Track id.
        track_id: String,
        /// Item id.
        item_id: String,
        /// New source in.
        new_src_in: f64,
        /// New source out.
        new_src_out: f64,
    },
    /// Change a clip's timeline placement.
    MoveClip {
        /// Track id.
        track_id: String,
        /// Item id.
        item_id: String,
        /// New timeline in.
        new_timeline_in: f64,
    },
    /// Set `locked_by_user` on a clip.
    PinClip {
        /// Track id.
        track_id: String,
        /// Item id.
        item_id: String,
    },
    /// Clear `locked_by_user` on a clip.
    UnpinClip {
        /// Track id.
        track_id: String,
        /// Item id.
        item_id: String,
    },
    /// Add a marker to a clip.
    AddMarker {
        /// Track id.
        track_id: String,
        /// Item id.
        item_id: String,
        /// Marker.
        marker: Marker,
    },
    /// Remove a marker by index. Inverse of `AddMarker`.
    RemoveMarker {
        /// Track id.
        track_id: String,
        /// Item id.
        item_id: String,
        /// Index into the clip's `markers[]` vector.
        marker_index: usize,
    },
    /// Add a standalone caption.
    AddCaption(Caption),
    /// Remove a standalone caption by index. Inverse of `AddCaption`.
    RemoveCaption {
        /// Index into `captions[]`.
        index: usize,
    },
    /// Replace every non-pinned clip in the given timeline range with a new
    /// list of clips and captions. This is the op the planner emits.
    ReplaceTimelineRange {
        /// Track to operate on.
        track_id: String,
        /// Inclusive start of the range to replace.
        timeline_in: f64,
        /// Exclusive end of the range to replace.
        timeline_out: f64,
        /// Replacement clips. Their `timeline_in` is relative to the *new*
        /// timeline, with the range starting at `timeline_in`.
        new_items: Vec<ClipItem>,
        /// New captions to insert in the range.
        new_captions: Vec<Caption>,
    },
    /// Restore a previously captured snapshot of a timeline range. Inverse
    /// of `ReplaceTimelineRange`.
    RestoreTimelineRange {
        /// Track id.
        track_id: String,
        /// Range start.
        timeline_in: f64,
        /// Range end.
        timeline_out: f64,
        /// Items to splice back in.
        items: Vec<TrackItem>,
        /// Captions to splice back in.
        captions: Vec<Caption>,
    },
    /// Update project-wide settings.
    SetProjectSettings(Project),
}

/// Append-only log of `Op`s.
#[derive(Debug, Default, Clone)]
pub struct OpLog {
    ops: Vec<Op>,
}

impl OpLog {
    /// Empty log.
    pub fn new() -> Self {
        Self::default()
    }

    /// Number of ops in the log.
    pub fn len(&self) -> usize {
        self.ops.len()
    }

    /// Is the log empty?
    pub fn is_empty(&self) -> bool {
        self.ops.is_empty()
    }

    /// All ops, in order.
    pub fn ops(&self) -> &[Op] {
        &self.ops
    }

    /// Append an op.
    pub fn push(&mut self, op: Op) {
        self.ops.push(op);
    }

    /// Read an `ops.jsonl` from disk.
    pub fn load(path: impl AsRef<Path>) -> crate::Result<Self> {
        let s = std::fs::read_to_string(path.as_ref())?;
        let mut log = Self::new();
        for (i, line) in s.lines().enumerate() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            let op: Op = serde_json::from_str(trimmed)
                .map_err(|e| crate::Error::Schema(format!("ops.jsonl line {}: {}", i + 1, e)))?;
            log.ops.push(op);
        }
        Ok(log)
    }

    /// Write the log to `ops.jsonl`. One JSON object per line.
    pub fn save(&self, path: impl AsRef<Path>) -> crate::Result<()> {
        use std::io::Write;
        let mut f = std::fs::File::create(path)?;
        for op in &self.ops {
            let line = serde_json::to_string(op)?;
            writeln!(f, "{line}")?;
        }
        Ok(())
    }

    /// Append a single op to an `ops.jsonl` file (atomic-ish: open in append
    /// mode and write one line).
    pub fn append_to_file(op: &Op, path: impl AsRef<Path>) -> crate::Result<()> {
        use std::io::Write;
        let mut f = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)?;
        let line = serde_json::to_string(op)?;
        writeln!(f, "{line}")?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opkind_serde_round_trip() {
        let op = Op::new(OpKind::AddTrack {
            track_id: "t_abc".into(),
            kind: TrackKind::Video,
        });
        let s = serde_json::to_string(&op).unwrap();
        let back: Op = serde_json::from_str(&s).unwrap();
        assert_eq!(op, back);
    }

    #[test]
    fn jsonl_round_trip() {
        let dir = tempdir();
        let path = dir.join("ops.jsonl");
        let mut log = OpLog::new();
        log.push(Op::new(OpKind::AddTrack {
            track_id: "t_v1".into(),
            kind: TrackKind::Video,
        }));
        log.push(Op::new(OpKind::AddTrack {
            track_id: "t_a1".into(),
            kind: TrackKind::Audio,
        }));
        log.save(&path).unwrap();
        let back = OpLog::load(&path).unwrap();
        assert_eq!(log.ops, back.ops);
    }

    fn tempdir() -> std::path::PathBuf {
        let p = std::env::temp_dir().join(format!("slop-core-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&p).unwrap();
        p
    }
}
