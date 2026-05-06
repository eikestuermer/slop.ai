//! HTTP client for any OpenAI-compatible chat-completions endpoint.

use crate::prompt::{build_messages, PromptStyle};
use serde_json::json;
use slop_core::{plan::Plan, repair::repair_plan, validator, Timeline};
use slop_score::PromptPack;
use thiserror::Error;

/// Configuration for the BYO LLM endpoint.
#[derive(Debug, Clone)]
pub struct EndpointConfig {
    /// Base URL, e.g. `http://localhost:11434/v1`. The trailing
    /// `/chat/completions` is appended automatically.
    pub base_url: String,
    /// Model name to send in the request body, e.g. `qwen3:8b`.
    pub model: String,
    /// Optional bearer token (leave blank for typical local endpoints).
    pub api_key: Option<String>,
    /// Request timeout in seconds.
    pub timeout_sec: u64,
    /// Sampling temperature (0..=2). 0 is recommended for the planner.
    pub temperature: f32,
}

impl Default for EndpointConfig {
    fn default() -> Self {
        Self {
            base_url: "http://localhost:11434/v1".to_string(),
            model: "qwen3:8b".to_string(),
            api_key: None,
            timeout_sec: 300,
            temperature: 0.0,
        }
    }
}

/// Errors that the planner client can surface.
#[derive(Debug, Error)]
pub enum PlannerError {
    /// HTTP transport error.
    #[error(transparent)]
    Http(#[from] reqwest::Error),
    /// The endpoint returned a non-200 status.
    #[error("endpoint returned {status}: {body}")]
    BadStatus {
        /// HTTP status code.
        status: u16,
        /// Response body.
        body: String,
    },
    /// The endpoint returned text that did not contain JSON.
    #[error("endpoint returned non-JSON content: {0}")]
    NonJsonContent(String),
    /// Schema validation failed both before and after a repair pass.
    #[error("plan validation failed after repair: {errors}")]
    SchemaInvalid {
        /// Error message.
        errors: String,
        /// Repair notes.
        repair_notes: Vec<String>,
    },
    /// JSON (de)serialization.
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

/// Result of a successful plan call.
#[derive(Debug, Clone)]
pub struct PlannerResult {
    /// The validated, repaired plan.
    pub plan: Plan,
    /// Notes from the repair pass, if any.
    pub repair_notes: Vec<String>,
    /// Raw model output for debugging / UI.
    pub raw_response: String,
}

/// Run a plan call.
///
/// `tl` is the current timeline; we use it to validate the plan's asset and
/// track references against the candidate set.
pub async fn plan(
    cfg: &EndpointConfig,
    pack: &PromptPack,
    tl: &Timeline,
    style: PromptStyle,
) -> Result<PlannerResult, PlannerError> {
    let plan_schema: serde_json::Value =
        serde_json::from_str(slop_core::validator::PLAN_SCHEMA)?;

    let messages = build_messages(pack, style);
    let body = json!({
        "model": cfg.model,
        "messages": messages,
        "temperature": cfg.temperature,
        "response_format": {
            "type": "json_schema",
            "json_schema": {
                "name": "SlopPlan",
                "strict": true,
                "schema": plan_schema
            }
        },
    });

    let url = format!("{}/chat/completions", cfg.base_url.trim_end_matches('/'));
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(cfg.timeout_sec))
        .build()?;
    let mut req = client.post(&url).json(&body);
    if let Some(key) = &cfg.api_key {
        if !key.is_empty() {
            req = req.bearer_auth(key);
        }
    }

    let resp = req.send().await?;
    let status = resp.status();
    let text = resp.text().await?;
    if !status.is_success() {
        return Err(PlannerError::BadStatus {
            status: status.as_u16(),
            body: text,
        });
    }

    let raw_value: serde_json::Value = serde_json::from_str(&text)?;
    let content = raw_value
        .pointer("/choices/0/message/content")
        .and_then(|v| v.as_str())
        .ok_or_else(|| PlannerError::NonJsonContent(text.clone()))?
        .to_string();

    // The model was constrained to JSON; parse it.
    let mut plan: Plan = serde_json::from_str(&content).map_err(|e| {
        PlannerError::NonJsonContent(format!("model content was not valid JSON: {e}: {content}"))
    })?;

    // Validate, then repair, then validate again.
    let mut repair_notes = Vec::new();
    if let Err(_e) = validator::validate_plan_semantics(&plan, tl) {
        repair_notes = repair_plan(&mut plan, tl);
        if let Err(e) = validator::validate_plan_semantics(&plan, tl) {
            return Err(PlannerError::SchemaInvalid {
                errors: e.to_string(),
                repair_notes,
            });
        }
    }

    Ok(PlannerResult {
        plan,
        repair_notes,
        raw_response: content,
    })
}
