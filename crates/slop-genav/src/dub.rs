//! Dubbing & translation pipeline.
//!
//! Pipeline = ASR -> machine translation -> TTS in target language with
//! cloned voice (from the consent ledger).
//!
//! ## Translation backends
//!
//! - **NLLB-200** (local) — Meta's No Language Left Behind, 200 languages,
//!   CC-BY-NC-4.0 weights.
//! - **SeamlessM4T-v2** (local) — direct audio-to-audio for the supported
//!   pairs; CC-BY-NC-4.0 weights.
//! - **DeepL** (hosted) opt-in.
//!
//! Slop AI runs translation through an OpenAI-compatible HTTP endpoint
//! that wraps the chosen model: typically a small `vllm` or
//! `text-generation-inference` server speaking the same `/v1/chat/completions`
//! shape we use for the planner. This means dubbing is "just another BYO
//! endpoint" the user configures alongside their planner endpoint.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Translation request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranslationRequest {
    /// Source text.
    pub text: String,
    /// BCP-47 source language (`"en"`, `"de"`, `"ja"` ...).
    pub source_lang: String,
    /// BCP-47 target language.
    pub target_lang: String,
    /// Optional context to bias style.
    pub context: Option<String>,
}

/// Translation provider errors.
#[derive(Debug, Error)]
pub enum TransError {
    /// HTTP transport.
    #[error(transparent)]
    Http(#[from] reqwest::Error),
    /// Provider rejected the request.
    #[error("{0}")]
    Provider(String),
    /// JSON.
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

/// Translation provider.
#[async_trait]
pub trait TranslationProvider: Send + Sync {
    /// Provider name.
    fn name(&self) -> &'static str;
    /// Translate.
    async fn translate(&self, req: &TranslationRequest) -> Result<String, TransError>;
}

/// OpenAI-compatible translation provider. Uses the chat-completions
/// endpoint with a constrained system prompt to guarantee single-line
/// translation output. Works with NLLB-200 / SeamlessM4T behind a vllm
/// gateway, or directly with any chat model that's good at translation
/// (Qwen3, Llama 3, Claude, GPT-5).
pub struct OpenAiCompatTranslator {
    /// Base URL.
    pub base_url: String,
    /// Model name.
    pub model: String,
    /// Bearer token.
    pub api_key: Option<String>,
}

#[async_trait]
impl TranslationProvider for OpenAiCompatTranslator {
    fn name(&self) -> &'static str {
        "openai-compat-translator"
    }
    async fn translate(&self, req: &TranslationRequest) -> Result<String, TransError> {
        let body = serde_json::json!({
            "model": self.model,
            "messages": [
                {"role": "system", "content": format!(
                    "You are a professional subtitle translator. Translate the user's input from {src} to {dst}. Output only the translation, on a single line, with no quoting or commentary. Preserve speaker tone and idiomatic register.",
                    src = req.source_lang, dst = req.target_lang
                )},
                {"role": "user", "content": req.context.clone().map(|c| format!("Context: {c}\n\nText: {}", req.text)).unwrap_or_else(|| req.text.clone())}
            ],
            "temperature": 0.2
        });
        let url = format!("{}/chat/completions", self.base_url.trim_end_matches('/'));
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(120))
            .build()?;
        let mut rq = client.post(&url).json(&body);
        if let Some(k) = &self.api_key {
            if !k.is_empty() {
                rq = rq.bearer_auth(k);
            }
        }
        let v: serde_json::Value = rq.send().await?.json().await?;
        let text = v["choices"][0]["message"]["content"]
            .as_str()
            .ok_or_else(|| TransError::Provider("no content".into()))?
            .to_string();
        Ok(text.trim().to_string())
    }
}

/// Dubbing pipeline orchestrator. Combines a translation provider with a
/// voice provider to produce per-segment dubbed audio.
pub struct DubPipeline {
    translator: Box<dyn TranslationProvider>,
}

impl DubPipeline {
    /// Construct.
    pub fn new(translator: Box<dyn TranslationProvider>) -> Self {
        Self { translator }
    }

    /// Translate a list of source-language captions into target-language
    /// captions. The voice synthesis pass happens separately via the
    /// `voice::VoiceProvider` so dubbed projects can reuse the consent
    /// ledger.
    pub async fn translate_captions<'a>(
        &self,
        segments: impl IntoIterator<Item = (&'a str, &'a str)>,
        source_lang: &str,
        target_lang: &str,
    ) -> Result<Vec<(String, String)>, TransError> {
        let mut out = Vec::new();
        for (segment_id, text) in segments {
            let translated = self
                .translator
                .translate(&TranslationRequest {
                    text: text.to_string(),
                    source_lang: source_lang.to_string(),
                    target_lang: target_lang.to_string(),
                    context: None,
                })
                .await?;
            out.push((segment_id.to_string(), translated));
        }
        Ok(out)
    }
}
