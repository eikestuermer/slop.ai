//! The agentic edit loop.

use crate::critic::{run_critic, CriticReport};
use crate::tools::{ToolCallRecord, ToolError, ToolRegistry};
use serde::{Deserialize, Serialize};
use slop_core::Timeline;
use slop_planner::EndpointConfig;
use std::time::Duration;
use thiserror::Error;

/// Knobs for the agent loop.
#[derive(Debug, Clone)]
pub struct AgentConfig {
    /// LLM endpoint.
    pub endpoint: EndpointConfig,
    /// Maximum tool-using iterations before bailing.
    pub max_iterations: u32,
    /// Stop iterating once the critic returns a score >= this threshold.
    pub critic_target: f32,
    /// Maximum critic loops (each runs a full agent loop with critic feedback).
    pub max_critic_loops: u32,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            endpoint: EndpointConfig::default(),
            max_iterations: 16,
            critic_target: 0.85,
            max_critic_loops: 3,
        }
    }
}

/// Final report from the agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentReport {
    /// All tool calls in execution order.
    pub tool_calls: Vec<ToolCallRecord>,
    /// All critic reports across loops.
    pub critic_reports: Vec<CriticReport>,
    /// Final assistant text (the "summary" the model leaves).
    pub final_message: String,
    /// Total iterations used (across all critic loops).
    pub iterations: u32,
    /// Did we hit the critic target?
    pub converged: bool,
}

/// Errors during the loop.
#[derive(Debug, Error)]
pub enum LoopError {
    /// HTTP transport error.
    #[error(transparent)]
    Http(#[from] reqwest::Error),
    /// Endpoint returned non-200.
    #[error("endpoint {status}: {body}")]
    BadStatus {
        /// HTTP status.
        status: u16,
        /// Body.
        body: String,
    },
    /// Tool failure.
    #[error(transparent)]
    Tool(#[from] ToolError),
    /// JSON.
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

/// Run the full agent loop.
///
/// `system_prompt` describes the user goal and any constraints. The model
/// is given the timeline as a context message and the tool registry as
/// callable functions.
pub async fn run_agent_loop(
    cfg: &AgentConfig,
    tl: &Timeline,
    tools: &ToolRegistry,
    system_prompt: &str,
) -> Result<AgentReport, LoopError> {
    let mut messages: Vec<serde_json::Value> = vec![
        serde_json::json!({"role": "system", "content": system_prompt}),
        serde_json::json!({
            "role": "user",
            "content": format!("Current timeline state:\n{}", serde_json::to_string_pretty(tl).unwrap_or_default())
        }),
    ];

    let mut tool_calls = Vec::new();
    let mut critic_reports = Vec::new();
    let mut total_iterations = 0u32;

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(cfg.endpoint.timeout_sec))
        .build()?;

    for critic_iter in 0..cfg.max_critic_loops.max(1) {
        for _ in 0..cfg.max_iterations {
            total_iterations += 1;
            let body = serde_json::json!({
                "model": cfg.endpoint.model,
                "messages": messages,
                "temperature": cfg.endpoint.temperature,
                "tools": tools.to_openai_tools(),
                "tool_choice": "auto"
            });
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
            let status = resp.status();
            let text = resp.text().await?;
            if !status.is_success() {
                return Err(LoopError::BadStatus {
                    status: status.as_u16(),
                    body: text,
                });
            }
            let parsed: serde_json::Value = serde_json::from_str(&text)?;
            let assistant_message = &parsed["choices"][0]["message"];
            messages.push(assistant_message.clone());

            let tool_calls_arr = assistant_message["tool_calls"].as_array();
            if tool_calls_arr.is_none() || tool_calls_arr.unwrap().is_empty() {
                // Final answer.
                break;
            }

            for call in tool_calls_arr.unwrap() {
                let name = call["function"]["name"].as_str().unwrap_or("").to_string();
                let raw_args = call["function"]["arguments"].as_str().unwrap_or("{}");
                let args: serde_json::Value =
                    serde_json::from_str(raw_args).unwrap_or(serde_json::json!({}));
                let result = match tools.invoke(&name, args.clone()).await {
                    Ok(v) => v,
                    Err(e) => serde_json::json!({"error": e.to_string()}),
                };
                tool_calls.push(ToolCallRecord {
                    name: name.clone(),
                    args,
                    result: result.clone(),
                });
                messages.push(serde_json::json!({
                    "role": "tool",
                    "tool_call_id": call["id"].as_str().unwrap_or(""),
                    "name": name,
                    "content": serde_json::to_string(&result).unwrap_or_default()
                }));
            }
        }

        // Critic pass.
        let critic = run_critic(cfg, tl).await.unwrap_or(CriticReport {
            score: 0.0,
            critique: "critic call failed".into(),
            suggestions: vec![],
        });
        let converged = critic.score >= cfg.critic_target;
        critic_reports.push(critic.clone());
        if converged {
            return Ok(AgentReport {
                tool_calls,
                critic_reports,
                final_message: extract_last_text(&messages),
                iterations: total_iterations,
                converged: true,
            });
        }
        if critic_iter + 1 < cfg.max_critic_loops {
            // Feed critic back to the model.
            messages.push(serde_json::json!({
                "role": "user",
                "content": format!(
                    "Self-critic score {:.2}. Address these critiques and try again:\n{}\nSuggestions: {}",
                    critic.score, critic.critique, critic.suggestions.join("; ")
                )
            }));
        }
    }

    Ok(AgentReport {
        tool_calls,
        critic_reports,
        final_message: extract_last_text(&messages),
        iterations: total_iterations,
        converged: false,
    })
}

fn extract_last_text(messages: &[serde_json::Value]) -> String {
    for m in messages.iter().rev() {
        if m["role"] == "assistant" {
            if let Some(s) = m["content"].as_str() {
                if !s.is_empty() {
                    return s.to_string();
                }
            }
        }
    }
    String::new()
}
