//! Self-critic pass.
//!
//! After the main agent loop produces a candidate timeline, we ask the same
//! (or a smaller) model to score the result against the original goal and
//! produce concrete critiques. The critic is constrained to a strict JSON
//! shape so its output is mechanical to parse.

use crate::loop_::AgentConfig;
use serde::{Deserialize, Serialize};
use slop_core::Timeline;

/// Score + critique.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CriticReport {
    /// Score in [0, 1].
    pub score: f32,
    /// Free-text critique.
    pub critique: String,
    /// Concrete suggestions (each a single sentence).
    pub suggestions: Vec<String>,
}

/// Run the critic against the current timeline.
pub async fn run_critic(cfg: &AgentConfig, tl: &Timeline) -> Result<CriticReport, reqwest::Error> {
    let schema = serde_json::json!({
        "type": "object",
        "required": ["score", "critique", "suggestions"],
        "additionalProperties": false,
        "properties": {
            "score": { "type": "number", "minimum": 0, "maximum": 1 },
            "critique": { "type": "string" },
            "suggestions": { "type": "array", "items": { "type": "string" } }
        }
    });

    let body = serde_json::json!({
        "model": cfg.endpoint.model,
        "messages": [
            {"role": "system", "content": "You are a senior video editor reviewing a rough cut. Score it 0..1 against the user's goal and emit concrete suggestions for the next iteration. Output strict JSON."},
            {"role": "user", "content": format!(
                "Timeline:\n{}",
                serde_json::to_string_pretty(tl).unwrap_or_default()
            )},
        ],
        "temperature": 0,
        "response_format": {
            "type": "json_schema",
            "json_schema": {
                "name": "CriticReport",
                "strict": true,
                "schema": schema
            }
        }
    });

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(cfg.endpoint.timeout_sec))
        .build()?;
    let url = format!(
        "{}/chat/completions",
        cfg.endpoint.base_url.trim_end_matches('/')
    );
    let mut req = client.post(&url).json(&body);
    if let Some(k) = &cfg.endpoint.api_key {
        if !k.is_empty() {
            req = req.bearer_auth(k);
        }
    }
    let resp = req.send().await?;
    let v: serde_json::Value = resp.json().await?;
    let content = v["choices"][0]["message"]["content"]
        .as_str()
        .unwrap_or("{}");
    Ok(serde_json::from_str(content).unwrap_or_default())
}
