//! whisper.cpp ASR backend.
//!
//! Compiled in only with `--features whisper-cpp`. Links the
//! [`whisper-rs`](https://crates.io/crates/whisper-rs) crate, which in turn
//! statically links a build of whisper.cpp.
//!
//! ## Audio prep
//!
//! whisper.cpp expects 16 kHz mono `f32` PCM. We shell out to ffmpeg to
//! decode any source format into that shape, into a temp file, then feed
//! the samples to the model.
//!
//! ## Chunking
//!
//! Whisper internally processes 30-second windows. Files longer than the
//! default chunker target (25s) are pre-split with silence-aware boundaries
//! by [`crate::chunk::chunk_audio`]; each chunk is transcribed independently
//! and the segments are stitched into a single transcript with timestamps
//! adjusted to the source timeline.

use crate::backend::{AsrBackend, AsrError, AsrJob, AsrOptions};
use crate::chunk::{chunk_audio, ChunkOptions};
use crate::transcript::{Segment, Transcript};
use async_trait::async_trait;
use std::path::PathBuf;
use uuid::Uuid;

/// whisper.cpp backend (feature-gated).
#[derive(Debug, Clone)]
pub struct WhisperCppBackend {
    /// Path to a downloaded GGUF model file.
    pub model_path: PathBuf,
    /// Number of CPU threads to use. `0` means "let whisper.cpp pick".
    pub threads: u32,
    /// Apply translation-to-English (whisper's `translate` flag).
    pub translate_to_english: bool,
}

impl WhisperCppBackend {
    /// Construct with a model file path and sensible defaults.
    pub fn new(model_path: impl Into<PathBuf>) -> Self {
        Self {
            model_path: model_path.into(),
            threads: 0,
            translate_to_english: false,
        }
    }
}

#[async_trait]
impl AsrBackend for WhisperCppBackend {
    fn name(&self) -> &'static str {
        "whisper-cpp"
    }

    async fn transcribe(&self, job: AsrJob, opts: &AsrOptions) -> Result<Transcript, AsrError> {
        // Decode source audio to 16 kHz mono f32 WAV.
        let pcm_path = decode_to_pcm16khz_mono(&job.input)
            .await
            .map_err(|e| AsrError::Backend(format!("decode audio: {e}")))?;

        // Slice into chunks. The chunker only enforces upper bounds; whisper
        // itself happily handles a single 30s window.
        let chunks = chunk_audio(job.duration_sec, None, &ChunkOptions::default());

        let mut all_segments: Vec<Segment> = Vec::new();
        let model_path = self.model_path.clone();
        let language = opts.language.clone();
        let translate = self.translate_to_english;
        let threads = self.threads;

        let pcm =
            read_wav_to_f32(&pcm_path).map_err(|e| AsrError::Backend(format!("read pcm: {e}")))?;
        let _ = std::fs::remove_file(&pcm_path);

        // Run whisper inside `spawn_blocking` so we don't stall the tokio
        // executor on a CPU-bound model.
        let result = tokio::task::spawn_blocking(move || -> Result<Vec<Segment>, String> {
            #[cfg(feature = "whisper-cpp")]
            {
                run_whisper(&model_path, &pcm, &chunks, &language, translate, threads)
            }
            #[cfg(not(feature = "whisper-cpp"))]
            {
                let _ = (model_path, pcm, chunks, language, translate, threads);
                Err("whisper-cpp feature not enabled in this build".into())
            }
        })
        .await
        .map_err(|e| AsrError::Backend(format!("spawn: {e}")))?
        .map_err(AsrError::Backend)?;
        all_segments.extend(result);

        Ok(Transcript {
            asset_id: job.asset_id,
            backend: "whisper-cpp".into(),
            model: self
                .model_path
                .file_name()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_else(|| "whisper".into()),
            language: Some(opts.language.clone()),
            segments: all_segments,
        })
    }
}

#[cfg(feature = "whisper-cpp")]
fn run_whisper(
    model_path: &std::path::Path,
    pcm: &[f32],
    chunks: &[crate::chunk::Chunk],
    language: &str,
    translate: bool,
    threads: u32,
) -> Result<Vec<Segment>, String> {
    use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

    let ctx = WhisperContext::new_with_params(
        model_path
            .to_str()
            .ok_or_else(|| "model path is not utf-8".to_string())?,
        WhisperContextParameters::default(),
    )
    .map_err(|e| format!("load model: {e}"))?;
    let mut state = ctx
        .create_state()
        .map_err(|e| format!("create state: {e}"))?;

    let mut all = Vec::new();
    for chunk in chunks {
        let from_sample = (chunk.start_sec * 16_000.0) as usize;
        let to_sample = ((chunk.end_sec * 16_000.0) as usize).min(pcm.len());
        if to_sample <= from_sample {
            continue;
        }
        let slice = &pcm[from_sample..to_sample];

        let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
        if !language.is_empty() && language != "auto" {
            params.set_language(Some(language));
        }
        params.set_translate(translate);
        if threads > 0 {
            params.set_n_threads(threads as i32);
        }
        params.set_print_special(false);
        params.set_print_progress(false);
        params.set_print_realtime(false);
        params.set_print_timestamps(false);

        state
            .full(params, slice)
            .map_err(|e| format!("whisper full: {e}"))?;

        let n_segs = state.full_n_segments().map_err(|e| e.to_string())?;
        for i in 0..n_segs {
            let text = state.full_get_segment_text(i).map_err(|e| e.to_string())?;
            let t0 = state.full_get_segment_t0(i).map_err(|e| e.to_string())? as f64 * 0.01; // whisper returns 10ms ticks
            let t1 = state.full_get_segment_t1(i).map_err(|e| e.to_string())? as f64 * 0.01;
            all.push(Segment {
                segment_id: format!("seg_{}", &Uuid::new_v4().simple().to_string()[..12]),
                start_sec: chunk.start_sec + t0,
                end_sec: chunk.start_sec + t1,
                speaker: None,
                text: text.trim().to_string(),
                confidence: None,
            });
        }
    }
    Ok(all)
}

async fn decode_to_pcm16khz_mono(input: &std::path::Path) -> Result<PathBuf, String> {
    use tokio::process::Command;
    let out = std::env::temp_dir().join(format!(
        "slop-asr-{}.wav",
        Uuid::new_v4().simple().to_string()
    ));
    let status = Command::new("ffmpeg")
        .args(["-y", "-i"])
        .arg(input)
        .args([
            "-vn",
            "-ac",
            "1",
            "-ar",
            "16000",
            "-f",
            "wav",
            "-acodec",
            "pcm_s16le",
        ])
        .arg(&out)
        .status()
        .await
        .map_err(|e| e.to_string())?;
    if !status.success() {
        return Err(format!("ffmpeg exited {}", status.code().unwrap_or(-1)));
    }
    Ok(out)
}

fn read_wav_to_f32(path: &std::path::Path) -> Result<Vec<f32>, String> {
    let mut reader = hound::WavReader::open(path).map_err(|e| e.to_string())?;
    let mut out = Vec::with_capacity(reader.duration() as usize);
    for s in reader.samples::<i16>() {
        let s = s.map_err(|e| e.to_string())?;
        out.push(s as f32 / i16::MAX as f32);
    }
    Ok(out)
}
