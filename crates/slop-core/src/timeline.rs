//! Canonical timeline types.
//!
//! These types are the Rust mirror of `packages/schemas/timeline.v1.json` and
//! are kept in sync by hand for V1. A future build step will codegen them via
//! `schemars` + `typify`. The schema is still the source of truth at runtime:
//! see [`crate::validator`].

use serde::{Deserialize, Serialize};

/// A whole project / app-state document.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Timeline {
    /// Always `roughcut.v1`.
    pub schema_version: String,
    /// Project-wide settings (frame rate, resolution, audio sample rate).
    pub project: Project,
    /// Imported source media.
    pub assets: Vec<Asset>,
    /// Composition tracks (video and audio).
    pub tracks: Vec<Track>,
    /// Standalone captions (separate from clip metadata).
    #[serde(default)]
    pub captions: Vec<Caption>,
    /// Standalone titles.
    #[serde(default)]
    pub titles: Vec<Title>,
    /// Which exports are enabled for this project.
    #[serde(default)]
    pub exports: Exports,
}

impl Timeline {
    /// Construct an empty project.
    pub fn empty() -> Self {
        Self {
            schema_version: crate::SCHEMA_VERSION.to_string(),
            project: Project::default_1080p_30(),
            assets: Vec::new(),
            tracks: Vec::new(),
            captions: Vec::new(),
            titles: Vec::new(),
            exports: Exports::default(),
        }
    }

    /// Look up an asset by id.
    pub fn asset(&self, id: &str) -> Option<&Asset> {
        self.assets.iter().find(|a| a.asset_id == id)
    }

    /// Look up a track by id.
    pub fn track(&self, id: &str) -> Option<&Track> {
        self.tracks.iter().find(|t| t.track_id == id)
    }

    /// Mutably look up a track by id.
    pub fn track_mut(&mut self, id: &str) -> Option<&mut Track> {
        self.tracks.iter_mut().find(|t| t.track_id == id)
    }

    /// Total timeline duration in seconds (max `timeline_out` across tracks).
    pub fn duration_sec(&self) -> f64 {
        self.tracks
            .iter()
            .flat_map(|t| t.items.iter().map(|i| i.timeline_out()))
            .fold(0.0, f64::max)
    }
}

/// Project-wide settings.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Project {
    /// Project frame rate.
    pub fps: f64,
    /// Project resolution in pixels.
    pub resolution: Resolution,
    /// Audio sample rate.
    pub sample_rate: u32,
}

impl Project {
    /// Convenience constructor for a 1920x1080 / 30fps / 48kHz project.
    pub fn default_1080p_30() -> Self {
        Self {
            fps: 30.0,
            resolution: Resolution { w: 1920, h: 1080 },
            sample_rate: 48_000,
        }
    }
}

/// A pixel resolution.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct Resolution {
    /// Width in pixels.
    pub w: u32,
    /// Height in pixels.
    pub h: u32,
}

/// A single source media file.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Asset {
    /// Stable asset id, e.g. `a_xxxxxxxxxxxx`.
    pub asset_id: String,
    /// Absolute or relative URI to the media file (file://, https://, etc).
    pub uri: String,
    /// Duration in seconds.
    pub duration_sec: f64,
    /// True if the file has at least one video stream.
    pub has_video: bool,
    /// True if the file has at least one audio stream.
    pub has_audio: bool,
    /// Frame rate of the source video, if known.
    #[serde(default)]
    pub fps: Option<f64>,
    /// Resolution of the source video, if known.
    #[serde(default)]
    pub resolution: Option<Resolution>,
    /// Reference to a stored transcript document.
    #[serde(default)]
    pub transcript_ref: Option<String>,
    /// Reference to a stored shot list document.
    #[serde(default)]
    pub shot_list_ref: Option<String>,
}

/// Track kind.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum TrackKind {
    /// Video track.
    Video,
    /// Audio track.
    Audio,
}

/// A single timeline track.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Track {
    /// Stable track id.
    pub track_id: String,
    /// Track kind.
    pub kind: TrackKind,
    /// Items on this track, sorted by `timeline_in`.
    pub items: Vec<TrackItem>,
}

/// A track item is either a clip referencing an asset or a gap.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum TrackItem {
    /// A clip referencing a source asset.
    Clip(ClipItem),
    /// An explicit empty range.
    Gap(GapItem),
}

impl TrackItem {
    /// Item id.
    pub fn id(&self) -> &str {
        match self {
            TrackItem::Clip(c) => &c.item_id,
            TrackItem::Gap(g) => &g.item_id,
        }
    }
    /// Timeline in, in seconds.
    pub fn timeline_in(&self) -> f64 {
        match self {
            TrackItem::Clip(c) => c.timeline_in,
            TrackItem::Gap(g) => g.timeline_in,
        }
    }
    /// Timeline out, in seconds.
    pub fn timeline_out(&self) -> f64 {
        match self {
            TrackItem::Clip(c) => c.timeline_out,
            TrackItem::Gap(g) => g.timeline_out,
        }
    }
}

/// A clip referencing a source asset.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ClipItem {
    /// Stable item id.
    pub item_id: String,
    /// Source asset id.
    pub asset_id: String,
    /// Source-time in.
    pub src_in: f64,
    /// Source-time out.
    pub src_out: f64,
    /// Timeline-time in.
    pub timeline_in: f64,
    /// Timeline-time out.
    pub timeline_out: f64,
    /// Playback speed scalar (1.0 = real time).
    #[serde(default = "one")]
    pub speed: f64,
    /// Per-clip effects.
    #[serde(default)]
    pub effects: Vec<Effect>,
    /// Per-clip markers.
    #[serde(default)]
    pub markers: Vec<Marker>,
    /// Editor metadata: scoring, locks, prompt provenance.
    #[serde(default)]
    pub metadata: ClipMetadata,
}

fn one() -> f64 {
    1.0
}

/// An explicit empty range.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GapItem {
    /// Stable item id.
    pub item_id: String,
    /// Timeline-time in.
    pub timeline_in: f64,
    /// Timeline-time out.
    pub timeline_out: f64,
}

/// Per-clip effect.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Effect {
    /// Effect kind.
    pub kind: EffectKind,
    /// Optional duration; meaning depends on the effect.
    #[serde(default)]
    pub duration_sec: Option<f64>,
}

/// V1 effect kinds. Intentionally tiny.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EffectKind {
    /// Audio + video fade up from black/silence.
    FadeIn,
    /// Audio + video fade down to black/silence.
    FadeOut,
    /// Cross dissolve into the next clip on the same track.
    CrossDissolve,
}

/// A single point-in-time marker on a clip or timeline.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Marker {
    /// Time in seconds.
    pub time_sec: f64,
    /// Human-readable label.
    pub label: String,
    /// Color hex. Defaults to grey.
    #[serde(default = "default_color")]
    pub color: String,
}

fn default_color() -> String {
    "#888".to_string()
}

/// Editor metadata for a clip.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct ClipMetadata {
    /// Why the planner picked this clip.
    #[serde(default)]
    pub selection_reason: Option<String>,
    /// Confidence score in [0, 1].
    #[serde(default)]
    pub score: Option<f64>,
    /// Set by the user to prevent regeneration.
    #[serde(default)]
    pub locked_by_user: bool,
    /// Originating prompt id.
    #[serde(default)]
    pub prompt_id: Option<String>,
}

/// A standalone caption.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Caption {
    /// Timeline-time in.
    pub timeline_in: f64,
    /// Timeline-time out.
    pub timeline_out: f64,
    /// Caption text.
    pub text: String,
    /// Originating transcript segment id.
    #[serde(default)]
    pub segment_id: Option<String>,
}

/// A standalone title (lower-thirds, supers).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Title {
    /// Timeline-time in.
    pub timeline_in: f64,
    /// Timeline-time out.
    pub timeline_out: f64,
    /// Title text.
    pub text: String,
    /// Visual style key.
    #[serde(default = "default_title_style")]
    pub style: String,
}

fn default_title_style() -> String {
    "lower-third".to_string()
}

/// Which exports are enabled.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Exports {
    /// Emit OTIO.
    #[serde(default = "yes")]
    pub otio: bool,
    /// Emit Premiere FCP7 XML.
    #[serde(default)]
    pub premiere_fcp7_xml: bool,
    /// Emit DaVinci Resolve FCPXML.
    #[serde(default)]
    pub resolve_fcpxml: bool,
    /// Emit Kdenlive .kdenlive XML.
    #[serde(default)]
    pub kdenlive: bool,
    /// Render an MP4 preview.
    #[serde(default = "yes")]
    pub mp4_preview: bool,
}

impl Default for Exports {
    fn default() -> Self {
        Self {
            otio: true,
            premiere_fcp7_xml: false,
            resolve_fcpxml: false,
            kdenlive: false,
            mp4_preview: true,
        }
    }
}

fn yes() -> bool {
    true
}
