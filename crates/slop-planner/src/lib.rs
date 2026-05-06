//! # slop-planner
//!
//! Talks to any OpenAI-compatible chat-completions endpoint and asks it to
//! produce a [`slop_core::Plan`] that picks clips from a candidate set.
//!
//! ## BYO endpoint
//!
//! Slop AI is local-first. The shipped UI lets the user point at any
//! OpenAI-compatible URL: Ollama (`http://localhost:11434/v1`), llama.cpp
//! server, LM Studio, vLLM, OpenAI itself, OpenRouter, or anything else
//! that speaks the same shape. We never hardcode a vendor.
//!
//! ## Strict outputs
//!
//! The planner uses two safety mechanisms in tandem:
//!
//! 1. The HTTP request includes `response_format = { type: "json_schema",
//!    json_schema: { ... } }`, which most modern endpoints (OpenAI,
//!    Ollama with structured outputs, llama.cpp's grammar mode) honor.
//! 2. Whatever JSON comes back is independently validated by
//!    `slop_core::validator::validate_plan_schema`. If validation fails,
//!    [`crate::repair`] runs a deterministic repair pass and validates
//!    again. If it still fails, we return the schema errors to the caller
//!    so they can show them in the UI; we do not silently mutate state.

#![deny(missing_docs)]

pub mod client;
pub mod prompt;

pub use client::{plan, EndpointConfig, PlannerError, PlannerResult};
pub use prompt::{build_messages, PromptStyle};
