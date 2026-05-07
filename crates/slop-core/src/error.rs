//! Crate-wide error types.

use thiserror::Error;

/// Result alias used throughout slop-core.
pub type Result<T, E = Error> = std::result::Result<T, E>;

/// Top-level error for slop-core.
#[derive(Debug, Error)]
pub enum Error {
    /// Schema validation failed against `timeline.v1.json`, `ops.v1.json`, or
    /// `plan.v1.json`.
    #[error("schema validation failed: {0}")]
    Schema(String),

    /// A clip references an `asset_id` that does not exist on the timeline.
    #[error("unknown asset id: {0}")]
    UnknownAsset(String),

    /// A clip references a track that does not exist.
    #[error("unknown track id: {0}")]
    UnknownTrack(String),

    /// A clip's source range falls outside the asset's duration.
    #[error("clip {item_id}: src range [{src_in}, {src_out}] outside asset duration {duration}")]
    SrcOutOfRange {
        /// The offending item id.
        item_id: String,
        /// Source in.
        src_in: f64,
        /// Source out.
        src_out: f64,
        /// Asset duration.
        duration: f64,
    },

    /// A clip's source range is non-positive.
    #[error("clip {0}: src_in must be < src_out")]
    SrcEmpty(String),

    /// Two clip items overlap on the same track and lane.
    #[error(
        "items {a} and {b} overlap on track {track}: ranges [{a_in},{a_out}] and [{b_in},{b_out}]"
    )]
    Overlap {
        /// The track id.
        track: String,
        /// First item id.
        a: String,
        /// Second item id.
        b: String,
        /// First item timeline_in.
        a_in: f64,
        /// First item timeline_out.
        a_out: f64,
        /// Second item timeline_in.
        b_in: f64,
        /// Second item timeline_out.
        b_out: f64,
    },

    /// JSON (de)serialization failure.
    #[error(transparent)]
    Json(#[from] serde_json::Error),

    /// I/O failure.
    #[error(transparent)]
    Io(#[from] std::io::Error),

    /// Generic invariant violation.
    #[error("invariant violated: {0}")]
    Invariant(String),
}
