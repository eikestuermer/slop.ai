//! # slop-vision
//!
//! Multi-modal planner support.
//!
//! Vision-capable LLMs in 2026 (Qwen2.5-VL, Llava-OneVision, Llama 3.2
//! Vision, GPT-4o, Gemini 2.5, Claude 3.7) accept image inputs in OpenAI
//! chat-completions shape: `{type: "image_url", image_url: {url: "data:..." }}`.
//! This crate produces those payloads for each candidate moment.
//!
//! ## Tiling strategy
//!
//! For each candidate moment we sample N frames (default: 3 — first,
//! middle, last) at the moment's source timestamps via ffmpeg's `select`
//! filter, scale to the model's native resolution (default: 448x448 to
//! match Qwen2.5-VL and Llava-Next), and emit them as base64 JPEG image
//! parts.
//!
//! Token budgets matter: each 448x448 image costs roughly 1k input tokens
//! on most vision models. We target a *visual budget* per request rather
//! than a per-clip frame count, so longer candidate sets get fewer frames
//! per clip.

#![deny(missing_docs)]

pub mod budget;
pub mod tile;

pub use budget::{plan_visual_budget, VisualBudget};
pub use tile::{frames_at_timestamps, FrameTile, TileOptions};
