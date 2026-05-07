//! Schema migrations.
//!
//! Each module is named for the version it migrates *from*. A migration
//! takes a parsed v(N) document and returns a v(N+1) document, and is
//! exercised by tests against fixture files in `examples/sample-projects/`.
//!
//! Migrations are append-only: once shipped they are never edited; if a
//! bug is found the fix is a *new* migration that runs after.
//!
//! ## V1 -> V2
//!
//! See [`v1_to_v2`]. Adds:
//! - compound clips, multicam groups, transitions as track items
//! - J/L offsets, speed curves, effect graph with keyframes
//! - styled captions and titles
//! - mixer + color pipeline at the project level
//!
//! V1 files load losslessly into V2: the migration sets new fields to their
//! defaults and converts the V1 `speed` scalar into a single-keyframe
//! `SpeedCurve`.

pub mod v1_to_v2;

pub use v1_to_v2::migrate_v1_to_v2;
