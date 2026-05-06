//! # slop-score
//!
//! Build a list of *candidate moments* and assign each one a confidence
//! score. The planner LLM is forbidden from discovering raw media; it only
//! ever picks from this candidate list.
//!
//! A "moment" is a `(asset_id, start_sec, end_sec)` range, derived from one
//! or more underlying signals:
//!
//! - **transcript segments** (`slop-asr`): natural sentence boundaries.
//! - **scene boundaries** (`slop-scenes`): visual shot changes.
//! - **silence boundaries**: low-amplitude regions in the waveform peaks.
//! - **lexical highlights**: keywords matching a small built-in pattern set
//!   (questions, numbers, named-entity-shaped capitals).
//! - **clean framing heuristics** (deferred to V1.1): face count + stability.
//!
//! For V1 the scorer is intentionally rule-based. Each feature contributes a
//! weight; the final score is the clamped weighted sum. We never train a
//! classifier on user content.

#![deny(missing_docs)]

pub mod features;
pub mod moment;
pub mod prompt_pack;
pub mod scorer;

pub use moment::{Moment, MomentBuilder};
pub use prompt_pack::{build_prompt_pack, PromptPack};
pub use scorer::{score_moments, ScoreWeights};
