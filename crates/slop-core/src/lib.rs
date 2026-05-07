//! # slop-core
//!
//! The canonical timeline schema, reversible op log, validator, and reducer
//! for Slop AI.
//!
//! ## Architectural pillars
//!
//! - The `Timeline` type is the *canonical* app-state. OTIO is derived from it.
//! - All mutations go through `Op`s recorded in `OpLog` (`ops.jsonl`).
//! - LLM-generated `Plan`s are validated by [`validator`] and corrected by
//!   [`repair`] before being converted to `Op`s.
//! - Every `Op` has an inverse, enabling undo/redo and crash recovery via
//!   log replay.

#![deny(missing_docs)]

pub mod error;
pub mod ids;
pub mod migrations;
pub mod ops;
pub mod plan;
pub mod reducer;
pub mod repair;
pub mod timeline;
pub mod validator;

pub use error::{Error, Result};
pub use ops::{Op, OpKind, OpLog};
pub use plan::Plan;
pub use reducer::apply;
pub use timeline::{
    Asset, Caption, ClipItem, ClipMetadata, Effect, EffectKind, GapItem, Marker, Project,
    Resolution, Timeline, Title, Track, TrackItem, TrackKind,
};

/// The schema version this build of slop-core understands.
pub const SCHEMA_VERSION: &str = "roughcut.v1";

/// The plan schema version this build of slop-core understands.
pub const PLAN_VERSION: &str = "roughcut_plan.v1";
