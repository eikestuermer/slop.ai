//! Tool registry for the agentic edit loop.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use thiserror::Error;

/// Errors a tool implementation can raise.
#[derive(Debug, Error)]
pub enum ToolError {
    /// Model called a tool we do not have registered.
    #[error("unknown tool: {0}")]
    UnknownTool(String),
    /// Tool arguments did not match the schema.
    #[error("invalid arguments for {tool}: {message}")]
    InvalidArgs {
        /// Tool name.
        tool: String,
        /// Detail.
        message: String,
    },
    /// Tool execution failed at runtime.
    #[error("{0}")]
    Runtime(String),
}

/// A single tool the model can call.
#[async_trait]
pub trait Tool: Send + Sync {
    /// Tool name (must be a valid identifier; matches what the model emits).
    fn name(&self) -> &'static str;
    /// One-line description shown to the model.
    fn description(&self) -> &'static str;
    /// JSON Schema for the tool's input.
    fn parameters_schema(&self) -> serde_json::Value;
    /// Execute the tool. Receives JSON arguments already validated against
    /// the schema.
    async fn invoke(&self, args: serde_json::Value) -> Result<serde_json::Value, ToolError>;
}

/// Registry mapping tool names to implementations.
pub struct ToolRegistry {
    tools: BTreeMap<&'static str, Box<dyn Tool>>,
}

impl ToolRegistry {
    /// Empty registry.
    pub fn new() -> Self {
        Self {
            tools: BTreeMap::new(),
        }
    }
    /// Register a tool. Panics on duplicate name.
    pub fn register(&mut self, tool: Box<dyn Tool>) {
        let name = tool.name();
        assert!(
            self.tools.insert(name, tool).is_none(),
            "duplicate tool name {name}"
        );
    }

    /// Tool names in registration order.
    pub fn names(&self) -> Vec<&'static str> {
        self.tools.keys().copied().collect()
    }

    /// Render the registered tools as the OpenAI `tools` array for chat
    /// completions.
    pub fn to_openai_tools(&self) -> Vec<serde_json::Value> {
        self.tools
            .values()
            .map(|t| {
                serde_json::json!({
                    "type": "function",
                    "function": {
                        "name": t.name(),
                        "description": t.description(),
                        "parameters": t.parameters_schema(),
                    }
                })
            })
            .collect()
    }

    /// Validate args against the tool's schema and invoke it.
    pub async fn invoke(
        &self,
        name: &str,
        args: serde_json::Value,
    ) -> Result<serde_json::Value, ToolError> {
        let tool = self
            .tools
            .get(name)
            .ok_or_else(|| ToolError::UnknownTool(name.to_string()))?;
        let schema = tool.parameters_schema();
        let compiled = jsonschema::JSONSchema::options()
            .with_draft(jsonschema::Draft::Draft202012)
            .compile(&schema)
            .map_err(|e| ToolError::InvalidArgs {
                tool: name.into(),
                message: format!("schema compile: {e}"),
            })?;
        if let Err(errors) = compiled.validate(&args) {
            let msg = errors.map(|e| e.to_string()).collect::<Vec<_>>().join("; ");
            return Err(ToolError::InvalidArgs {
                tool: name.into(),
                message: msg,
            });
        }
        tool.invoke(args).await
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Built-in: pin a clip on a track.
pub struct PinClipTool;

#[async_trait]
impl Tool for PinClipTool {
    fn name(&self) -> &'static str {
        "pin_clip"
    }
    fn description(&self) -> &'static str {
        "Mark a clip as user-locked so subsequent regenerations don't replace it."
    }
    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "required": ["track_id", "item_id"],
            "additionalProperties": false,
            "properties": {
                "track_id": { "type": "string" },
                "item_id": { "type": "string" }
            }
        })
    }
    async fn invoke(&self, args: serde_json::Value) -> Result<serde_json::Value, ToolError> {
        let track_id = args["track_id"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArgs {
                tool: self.name().into(),
                message: "track_id".into(),
            })?
            .to_string();
        let item_id = args["item_id"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArgs {
                tool: self.name().into(),
                message: "item_id".into(),
            })?
            .to_string();
        Ok(serde_json::json!({
            "ok": true,
            "op": "PinClip",
            "track_id": track_id,
            "item_id": item_id
        }))
    }
}

/// Test scaffolding for downstream consumers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallRecord {
    /// Tool name.
    pub name: String,
    /// Arguments the model emitted.
    pub args: serde_json::Value,
    /// Result the tool returned (or error string).
    pub result: serde_json::Value,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn registry_invokes_known_tool() {
        let mut reg = ToolRegistry::new();
        reg.register(Box::new(PinClipTool));
        let r = reg
            .invoke(
                "pin_clip",
                serde_json::json!({ "track_id": "v1", "item_id": "c1" }),
            )
            .await
            .unwrap();
        assert_eq!(r["op"], "PinClip");
        assert_eq!(r["item_id"], "c1");
    }

    #[tokio::test]
    async fn registry_rejects_invalid_args() {
        let mut reg = ToolRegistry::new();
        reg.register(Box::new(PinClipTool));
        let r = reg.invoke("pin_clip", serde_json::json!({})).await;
        assert!(matches!(r, Err(ToolError::InvalidArgs { .. })));
    }

    #[test]
    fn renders_openai_tools_payload() {
        let mut reg = ToolRegistry::new();
        reg.register(Box::new(PinClipTool));
        let v = reg.to_openai_tools();
        assert_eq!(v.len(), 1);
        assert_eq!(v[0]["function"]["name"], "pin_clip");
    }

    #[tokio::test]
    async fn registry_returns_unknown_tool_for_unregistered_name() {
        let reg = ToolRegistry::new();
        let r = reg.invoke("nope", serde_json::json!({})).await;
        assert!(matches!(r, Err(ToolError::UnknownTool(s)) if s == "nope"));
    }

    #[tokio::test]
    async fn registry_rejects_extra_properties() {
        let mut reg = ToolRegistry::new();
        reg.register(Box::new(PinClipTool));
        let r = reg
            .invoke(
                "pin_clip",
                serde_json::json!({ "track_id": "v1", "item_id": "c1", "extra": "bad" }),
            )
            .await;
        assert!(matches!(r, Err(ToolError::InvalidArgs { .. })));
    }

    #[test]
    fn registry_names_lists_registered_tools() {
        let mut reg = ToolRegistry::new();
        reg.register(Box::new(PinClipTool));
        let names = reg.names();
        assert_eq!(names, vec!["pin_clip"]);
    }
}
