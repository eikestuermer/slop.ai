//! Generative B-roll providers.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use thiserror::Error;

/// One B-roll generation request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BRollRequest {
    /// Text prompt.
    pub prompt: String,
    /// Optional negative prompt.
    pub negative_prompt: Option<String>,
    /// Target duration in seconds.
    pub duration_sec: f64,
    /// Target resolution.
    pub resolution: (u32, u32),
    /// Target frame rate.
    pub fps: f64,
    /// Optional seed for reproducibility.
    pub seed: Option<u64>,
    /// Optional reference image (path) for image-to-video flows.
    pub reference_image: Option<PathBuf>,
}

/// Provider errors.
#[derive(Debug, Error)]
pub enum BRollError {
    /// HTTP transport.
    #[error(transparent)]
    Http(#[from] reqwest::Error),
    /// Provider rejected the request.
    #[error("{0}")]
    Provider(String),
    /// Generation timed out.
    #[error("timed out after {0}s")]
    Timeout(u64),
    /// I/O.
    #[error(transparent)]
    Io(#[from] std::io::Error),
    /// JSON.
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

/// A backend that can generate B-roll.
#[async_trait]
pub trait BRollProvider: Send + Sync {
    /// Provider name (`"comfyui"`, `"runway"`, ...).
    fn name(&self) -> &'static str;
    /// Submit a request and return the path to the generated MP4.
    async fn generate(&self, req: &BRollRequest) -> Result<PathBuf, BRollError>;
}

/// ComfyUI provider.
///
/// Talks to a running ComfyUI server (`http://localhost:8188` by default)
/// using its `/prompt` queue API. The provider holds a workflow template
/// (a JSON graph exported from the ComfyUI UI), substitutes the user
/// prompt + duration + seed into the appropriate node parameters, and
/// polls `/history/<prompt_id>` until completion.
pub struct ComfyUiProvider {
    /// Base URL of the ComfyUI server.
    pub base_url: String,
    /// Workflow JSON template (exported via "Save (API Format)" in ComfyUI).
    pub workflow_template: serde_json::Value,
    /// Node IDs to substitute the prompt / duration / seed into.
    pub bindings: ComfyBindings,
    /// Where to write the resulting MP4.
    pub output_dir: PathBuf,
    /// Request timeout in seconds.
    pub timeout_sec: u64,
}

/// Map from ComfyUI workflow node IDs to the parameters Slop AI fills in.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComfyBindings {
    /// Node ID + input name for the positive prompt.
    pub prompt_node: (String, String),
    /// Optional node + input for negative prompt.
    pub negative_prompt_node: Option<(String, String)>,
    /// Node + input for the seed.
    pub seed_node: Option<(String, String)>,
    /// Node + input for the duration in frames.
    pub frames_node: Option<(String, String)>,
}

#[async_trait]
impl BRollProvider for ComfyUiProvider {
    fn name(&self) -> &'static str {
        "comfyui"
    }
    async fn generate(&self, req: &BRollRequest) -> Result<PathBuf, BRollError> {
        let mut workflow = self.workflow_template.clone();
        bind_node(
            &mut workflow,
            &self.bindings.prompt_node.0,
            &self.bindings.prompt_node.1,
            serde_json::Value::String(req.prompt.clone()),
        );
        if let (Some(neg), Some((node, input))) =
            (&req.negative_prompt, &self.bindings.negative_prompt_node)
        {
            bind_node(
                &mut workflow,
                node,
                input,
                serde_json::Value::String(neg.clone()),
            );
        }
        if let (Some(seed), Some((node, input))) = (req.seed, &self.bindings.seed_node) {
            bind_node(
                &mut workflow,
                node,
                input,
                serde_json::Value::Number(seed.into()),
            );
        }
        if let Some((node, input)) = &self.bindings.frames_node {
            let frames = (req.duration_sec * req.fps).round() as u64;
            bind_node(
                &mut workflow,
                node,
                input,
                serde_json::Value::Number(frames.into()),
            );
        }

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(self.timeout_sec))
            .build()?;

        let resp: serde_json::Value = client
            .post(format!("{}/prompt", self.base_url.trim_end_matches('/')))
            .json(&serde_json::json!({"prompt": workflow}))
            .send()
            .await?
            .json()
            .await?;
        let prompt_id = resp["prompt_id"]
            .as_str()
            .ok_or_else(|| BRollError::Provider("no prompt_id in response".into()))?
            .to_string();

        // Poll for completion.
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(self.timeout_sec);
        loop {
            if std::time::Instant::now() > deadline {
                return Err(BRollError::Timeout(self.timeout_sec));
            }
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
            let history: serde_json::Value = client
                .get(format!(
                    "{}/history/{}",
                    self.base_url.trim_end_matches('/'),
                    prompt_id
                ))
                .send()
                .await?
                .json()
                .await?;
            if let Some(entry) = history.get(&prompt_id) {
                let outputs = &entry["outputs"];
                if let Some(out_node) = outputs.as_object().and_then(|o| o.values().next()) {
                    if let Some(videos) = out_node["gifs"]
                        .as_array()
                        .or(out_node["videos"].as_array())
                    {
                        if let Some(first) = videos.first() {
                            if let Some(filename) = first["filename"].as_str() {
                                let url = format!(
                                    "{}/view?filename={}&subfolder={}&type={}",
                                    self.base_url.trim_end_matches('/'),
                                    urlencode(filename),
                                    urlencode(first["subfolder"].as_str().unwrap_or("")),
                                    urlencode(first["type"].as_str().unwrap_or("output")),
                                );
                                let bytes = client.get(&url).send().await?.bytes().await?;
                                std::fs::create_dir_all(&self.output_dir)?;
                                let dest = self
                                    .output_dir
                                    .join(format!("broll_{}.mp4", uuid::Uuid::new_v4().simple()));
                                std::fs::write(&dest, &bytes)?;
                                return Ok(dest);
                            }
                        }
                    }
                }
            }
        }
    }
}

fn bind_node(
    workflow: &mut serde_json::Value,
    node_id: &str,
    input: &str,
    value: serde_json::Value,
) {
    if let Some(node) = workflow
        .get_mut(node_id)
        .and_then(|n| n.get_mut("inputs"))
        .and_then(|i| i.as_object_mut())
    {
        node.insert(input.to_string(), value);
    }
}

fn urlencode(s: &str) -> String {
    s.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                (b as char).to_string()
            }
            _ => format!("%{b:02X}"),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bind_node_replaces_input() {
        let mut wf = serde_json::json!({
            "5": { "inputs": { "text": "old prompt" } }
        });
        bind_node(
            &mut wf,
            "5",
            "text",
            serde_json::Value::String("new".into()),
        );
        assert_eq!(wf["5"]["inputs"]["text"], "new");
    }

    #[test]
    fn urlencode_handles_specials() {
        assert_eq!(urlencode("hello world"), "hello%20world");
        assert_eq!(urlencode("ok-name_1.0~"), "ok-name_1.0~");
    }

    #[test]
    fn urlencode_matches_encodeuricomponent_semantics() {
        // RFC 3986 unreserved chars: A-Z a-z 0-9 - _ . ~
        // encodeURIComponent does NOT encode these; everything else IS encoded.
        // Some chars JS encodeURIComponent treats specially: !, ', (, ), *
        // RFC strictly says these should be percent-encoded — we follow RFC.
        assert_eq!(urlencode("a/b"), "a%2Fb");
        assert_eq!(urlencode("a&b=c"), "a%26b%3Dc");
        assert_eq!(
            urlencode("subfolder?type=output"),
            "subfolder%3Ftype%3Doutput"
        );
        assert_eq!(urlencode("100%"), "100%25");
        // Non-ASCII bytes are percent-encoded byte-by-byte (UTF-8).
        // 'é' = U+00E9 = 0xC3 0xA9 in UTF-8.
        assert_eq!(urlencode("é"), "%C3%A9");
    }

    #[test]
    fn bind_node_no_op_on_missing_node() {
        let mut wf = serde_json::json!({"5": {"inputs": {"text": "x"}}});
        let before = wf.clone();
        bind_node(
            &mut wf,
            "999",
            "text",
            serde_json::Value::String("ignored".into()),
        );
        assert_eq!(wf, before);
    }

    #[test]
    fn bind_node_inserts_new_input_key() {
        let mut wf = serde_json::json!({"5": {"inputs": {"text": "x"}}});
        bind_node(&mut wf, "5", "seed", serde_json::Value::Number(42.into()));
        assert_eq!(wf["5"]["inputs"]["seed"], 42);
        assert_eq!(wf["5"]["inputs"]["text"], "x");
    }
}
