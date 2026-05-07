//! # slop-genav
//!
//! Generative audio/video integrations.
//!
//! All providers are BYO-first. Slop AI never bundles weights; users
//! configure endpoints (local or hosted) via the project settings UI.
//!
//! ## Providers
//!
//! ### Video (B-roll)
//! - **ComfyUI** (local) — workflow JSON sent to a running ComfyUI server.
//!   Default workflow targets Wan2.1 (Apache-2.0 weights). Alternative
//!   workflows ship for Mochi-1 (Apache-2.0) and HunyuanVideo
//!   (Tencent license). See [`broll::ComfyUiProvider`].
//! - **InvokeAI** (local) — same shape, different endpoint.
//! - **Runway / Pika / Luma / Veo** (hosted) — explicit per-project opt-in.
//!
//! ### TTS / voice cloning
//! - **XTTS-v2** (local, via Coqui-TTS server). Cloning needs a 6-second
//!   sample.
//! - **F5-TTS** (local). Newer than XTTS, higher fidelity, no fine-tune.
//! - **Kokoro** (local). Tiny, very fast, no cloning.
//! - **ElevenLabs / OpenAI TTS** (hosted) opt-in.
//!
//! ### Translation
//! - **NLLB-200** (local) — Meta's no-language-left-behind, 200 languages.
//! - **SeamlessM4T-v2** (local) — Meta's audio-to-audio direct dub.
//! - **DeepL / Google Translate** (hosted) opt-in.
//!
//! ## Consent
//!
//! Voice cloning has a hard consent gate: the project must contain a
//! `voice_consent.json` listing each cloned speaker, the provenance of the
//! sample, and the consent grant. The cloning function refuses to run
//! without it. See [`voice::ConsentLedger`].

#![deny(missing_docs)]

pub mod broll;
pub mod dub;
pub mod voice;

pub use broll::{BRollProvider, BRollRequest, ComfyUiProvider};
pub use dub::{DubPipeline, TranslationProvider};
pub use voice::{ConsentLedger, ConsentRecord, VoiceProvider, XttsProvider};
