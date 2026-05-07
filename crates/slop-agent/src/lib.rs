//! # slop-agent
//!
//! Agentic edit loop. The planner stops being a one-shot JSON returner and
//! becomes an agent that calls tools, observes results, and iterates.
//!
//! ## Protocol
//!
//! We use the OpenAI tool-call protocol introduced in 2024 and now
//! supported by every major endpoint (OpenAI, Ollama, llama.cpp server,
//! Anthropic via compat layers, vLLM, OpenRouter). Each iteration:
//!
//! 1. We send the current message log + tool schema to the model.
//! 2. Model emits zero or more `tool_calls` in its response.
//! 3. We execute each tool call locally and append `tool` messages with
//!    the JSON results.
//! 4. We loop until the model returns a final assistant message *without*
//!    tool calls, or until `max_iterations` is hit.
//! 5. A self-critic pass scores the final timeline; if the score is below
//!    threshold and we have iterations left, we feed the critique back as
//!    a fresh user message.
//!
//! ## Tools
//!
//! See [`tools::Tool`]. The V2.0 toolset:
//!
//! - `pin_clip` / `unpin_clip`: protect a clip from regeneration.
//! - `replace_range`: replace a non-pinned timeline range with new clips.
//! - `add_caption`: add a caption.
//! - `render_preview`: render a low-res MP4 preview and return its path.
//! - `score_critic`: invoke the self-critic against the current state.
//!
//! Every tool's input schema is a real JSON Schema; the model's tool-call
//! arguments are validated before execution.

#![deny(missing_docs)]

pub mod critic;
pub mod loop_;
pub mod tools;

pub use critic::{run_critic, CriticReport};
pub use loop_::{run_agent_loop, AgentConfig, AgentReport};
pub use tools::{Tool, ToolError, ToolRegistry};
