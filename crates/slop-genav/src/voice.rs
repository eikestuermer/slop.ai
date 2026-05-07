//! TTS / voice cloning + consent ledger.
//!
//! Voice cloning is the highest-stakes feature in V2.0. Slop AI requires a
//! per-project consent ledger before any cloning runs. The ledger is a
//! plain JSON file at the project root (`voice_consent.json`); it lists
//! each speaker, the source of the reference sample, the consent
//! statement, the date, and an optional cryptographic signature.
//!
//! The provider refuses to clone any speaker not in the ledger.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use thiserror::Error;

/// One consent record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsentRecord {
    /// Speaker identifier used in the timeline (e.g. `"S0"`, `"alex"`).
    pub speaker_id: String,
    /// Display name of the consenting individual.
    pub display_name: String,
    /// Path or URI to the reference audio sample.
    pub reference_uri: String,
    /// SHA-256 of the reference sample (to detect tampering).
    pub reference_sha256: String,
    /// The exact consent statement the individual agreed to.
    pub consent_statement: String,
    /// Date/time of the grant.
    pub granted_at: DateTime<Utc>,
    /// Optional revocation date.
    pub revoked_at: Option<DateTime<Utc>>,
    /// Optional detached signature (e.g. base64 ed25519 over the rest of the record).
    pub signature: Option<String>,
}

/// Project-level consent ledger.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ConsentLedger {
    /// Records.
    pub records: Vec<ConsentRecord>,
}

impl ConsentLedger {
    /// Load `voice_consent.json` if present; returns an empty ledger
    /// otherwise. We never auto-create the ledger file; refusing to clone
    /// when the file is missing is the safety property.
    pub fn load(project_root: &Path) -> std::io::Result<Self> {
        let p = project_root.join("voice_consent.json");
        if !p.is_file() {
            return Ok(Self::default());
        }
        let s = std::fs::read_to_string(&p)?;
        let ledger: Self = serde_json::from_str(&s)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        Ok(ledger)
    }

    /// Is this speaker currently allowed to be cloned?
    pub fn is_allowed(&self, speaker_id: &str) -> bool {
        self.records
            .iter()
            .any(|r| r.speaker_id == speaker_id && r.revoked_at.is_none())
    }
}

/// Voice provider errors.
#[derive(Debug, Error)]
pub enum VoiceError {
    /// Speaker is not in the consent ledger.
    #[error("speaker {0} is not in the consent ledger; cloning refused")]
    ConsentMissing(String),
    /// HTTP transport.
    #[error(transparent)]
    Http(#[from] reqwest::Error),
    /// Provider rejected the request.
    #[error("{0}")]
    Provider(String),
    /// I/O.
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

/// A backend that can synthesize speech (with or without cloning).
#[async_trait]
pub trait VoiceProvider: Send + Sync {
    /// Provider name.
    fn name(&self) -> &'static str;
    /// Generate a single-line waveform for `text` in the speaker's voice.
    async fn synthesize(
        &self,
        text: &str,
        speaker_id: &str,
        ledger: &ConsentLedger,
        out_wav: &Path,
    ) -> Result<(), VoiceError>;
}

/// XTTS-v2 provider (via Coqui-TTS server).
pub struct XttsProvider {
    /// Base URL of the Coqui server (default `http://localhost:5002`).
    pub base_url: String,
    /// Map from `speaker_id` -> path to reference WAV.
    pub speaker_refs: std::collections::BTreeMap<String, PathBuf>,
    /// Language code.
    pub language: String,
}

#[async_trait]
impl VoiceProvider for XttsProvider {
    fn name(&self) -> &'static str {
        "xtts-v2"
    }
    async fn synthesize(
        &self,
        text: &str,
        speaker_id: &str,
        ledger: &ConsentLedger,
        out_wav: &Path,
    ) -> Result<(), VoiceError> {
        if !ledger.is_allowed(speaker_id) {
            return Err(VoiceError::ConsentMissing(speaker_id.to_string()));
        }
        let speaker_wav = self
            .speaker_refs
            .get(speaker_id)
            .ok_or_else(|| VoiceError::Provider(format!("no reference WAV for {speaker_id}")))?;
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(300))
            .build()?;
        let body = reqwest::multipart::Form::new()
            .text("text", text.to_string())
            .text("language", self.language.clone())
            .file("speaker_wav", speaker_wav)
            .await?;
        let resp = client
            .post(format!("{}/api/tts", self.base_url.trim_end_matches('/')))
            .multipart(body)
            .send()
            .await?
            .error_for_status()?;
        let bytes = resp.bytes().await?;
        if let Some(parent) = out_wav.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(out_wav, &bytes)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_ledger_disallows() {
        let l = ConsentLedger::default();
        assert!(!l.is_allowed("S0"));
    }

    #[test]
    fn revoked_record_disallows() {
        let l = ConsentLedger {
            records: vec![ConsentRecord {
                speaker_id: "S0".into(),
                display_name: "Alex".into(),
                reference_uri: "file:///r.wav".into(),
                reference_sha256: "deadbeef".into(),
                consent_statement: "I consent to AI cloning of my voice for this project.".into(),
                granted_at: Utc::now(),
                revoked_at: Some(Utc::now()),
                signature: None,
            }],
        };
        assert!(!l.is_allowed("S0"));
    }

    #[test]
    fn active_record_is_allowed() {
        let l = ConsentLedger {
            records: vec![ConsentRecord {
                speaker_id: "S1".into(),
                display_name: "Sam".into(),
                reference_uri: "file:///r.wav".into(),
                reference_sha256: "00".into(),
                consent_statement: "...".into(),
                granted_at: Utc::now(),
                revoked_at: None,
                signature: None,
            }],
        };
        assert!(l.is_allowed("S1"));
    }

    #[test]
    fn ledger_load_round_trip() {
        let dir =
            std::env::temp_dir().join(format!("slop-consent-{}", uuid::Uuid::new_v4().simple()));
        std::fs::create_dir_all(&dir).unwrap();
        let l = ConsentLedger {
            records: vec![ConsentRecord {
                speaker_id: "S0".into(),
                display_name: "Alex".into(),
                reference_uri: "file:///r.wav".into(),
                reference_sha256: "abcd".into(),
                consent_statement: "I consent.".into(),
                granted_at: Utc::now(),
                revoked_at: None,
                signature: None,
            }],
        };
        let path = dir.join("voice_consent.json");
        std::fs::write(&path, serde_json::to_string_pretty(&l).unwrap()).unwrap();
        let loaded = ConsentLedger::load(&dir).unwrap();
        assert_eq!(loaded.records.len(), 1);
        assert_eq!(loaded.records[0].speaker_id, "S0");
        assert!(loaded.is_allowed("S0"));
    }

    #[test]
    fn ledger_load_missing_file_returns_empty() {
        let dir = std::env::temp_dir().join(format!(
            "slop-consent-empty-{}",
            uuid::Uuid::new_v4().simple()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let loaded = ConsentLedger::load(&dir).unwrap();
        assert!(loaded.records.is_empty());
        assert!(!loaded.is_allowed("anyone"));
    }
}
