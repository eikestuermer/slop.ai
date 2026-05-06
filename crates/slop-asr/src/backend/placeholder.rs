//! Pure-Rust placeholder ASR backend.
//!
//! This backend never recognizes speech. It exists so:
//!
//! 1. CI builds and tests work without a C++ toolchain.
//! 2. Developers without whisper.cpp installed can still exercise the
//!    candidate-builder, planner, and render pipeline against synthetic
//!    transcripts.
//!
//! It works by:
//!
//! - extracting waveform peaks via [`slop_media::generate_waveform_peaks`],
//! - chunking the audio with [`crate::chunk::chunk_audio`],
//! - emitting one `Segment` per chunk with empty `text` and a fixed
//!   `confidence` of `0.0`.
//!
//! Because text is empty, downstream lexical scoring will be a no-op, but
//! shot detection, silence boundaries, and duration-based features still
//! produce useful candidates.

use crate::backend::{AsrBackend, AsrError, AsrJob, AsrOptions};
use crate::chunk::{chunk_audio, ChunkOptions};
use crate::transcript::{Segment, Transcript};
use async_trait::async_trait;
use slop_media::{generate_waveform_peaks, WaveformOptions};
use uuid::Uuid;

/// The placeholder backend.
#[derive(Debug, Default, Clone)]
pub struct PlaceholderBackend;

#[async_trait]
impl AsrBackend for PlaceholderBackend {
    fn name(&self) -> &'static str {
        "placeholder"
    }

    async fn transcribe(
        &self,
        job: AsrJob,
        opts: &AsrOptions,
    ) -> Result<Transcript, AsrError> {
        let peaks = if opts.silence_chunk {
            // Best-effort: if the user has no ffmpeg installed, fall through
            // to time-only chunking instead of failing.
            generate_waveform_peaks(&job.input, &WaveformOptions::default())
                .await
                .ok()
        } else {
            None
        };

        let chunks = chunk_audio(job.duration_sec, peaks.as_ref(), &ChunkOptions::default());
        let segments = chunks
            .into_iter()
            .map(|c| Segment {
                segment_id: format!("seg_{}", Uuid::new_v4().simple().to_string()[..12].to_string()),
                start_sec: c.start_sec,
                end_sec: c.end_sec,
                speaker: None,
                text: String::new(),
                confidence: Some(0.0),
            })
            .collect();

        Ok(Transcript {
            asset_id: job.asset_id,
            backend: "placeholder".into(),
            model: "placeholder".into(),
            language: Some(opts.language.clone()),
            segments,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn placeholder_emits_segments_for_long_input() {
        let b = PlaceholderBackend;
        // We don't have a real file, but transcribe() on a missing file
        // should still produce time-only segments because peaks extraction
        // fails gracefully.
        let job = AsrJob {
            asset_id: "a1".into(),
            input: std::path::PathBuf::from("/nonexistent.mp4"),
            duration_sec: 90.0,
        };
        let opts = AsrOptions::default();
        let t = b.transcribe(job, &opts).await.unwrap();
        assert!(t.segments.len() >= 3);
        assert!(t.segments.iter().all(|s| s.text.is_empty()));
        assert_eq!(t.backend, "placeholder");
    }
}
