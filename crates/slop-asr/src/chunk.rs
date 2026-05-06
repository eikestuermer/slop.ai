//! Audio chunking for ASR.
//!
//! Whisper-family models work best on chunks no longer than 30 seconds. We
//! pre-chunk long audio with simple silence-aware boundaries derived from
//! the waveform peaks computed by [`slop_media::waveform`]. Each chunk is a
//! `[start_sec, end_sec)` range with the boundary placed at the quietest
//! moment near the target chunk size.

use slop_media::WaveformPeaks;

/// A single chunk to feed to the ASR backend.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Chunk {
    /// Start time in seconds.
    pub start_sec: f64,
    /// End time in seconds.
    pub end_sec: f64,
}

/// Configuration for [`chunk_audio`].
#[derive(Debug, Clone, Copy)]
pub struct ChunkOptions {
    /// Target chunk length in seconds.
    pub target_sec: f64,
    /// Maximum chunk length in seconds.
    pub max_sec: f64,
    /// Minimum chunk length; smaller trailing chunks are merged into the
    /// previous one.
    pub min_sec: f64,
}

impl Default for ChunkOptions {
    fn default() -> Self {
        Self {
            target_sec: 25.0,
            max_sec: 30.0,
            min_sec: 5.0,
        }
    }
}

/// Split `[0, total_sec)` into chunks. If `peaks` is provided, snap each
/// boundary to the quietest local moment near the target.
pub fn chunk_audio(
    total_sec: f64,
    peaks: Option<&WaveformPeaks>,
    opts: &ChunkOptions,
) -> Vec<Chunk> {
    if total_sec <= opts.max_sec {
        return vec![Chunk {
            start_sec: 0.0,
            end_sec: total_sec,
        }];
    }

    let mut chunks = Vec::new();
    let mut t = 0.0_f64;
    while t < total_sec - 1e-6 {
        let target_end = (t + opts.target_sec).min(total_sec);
        let max_end = (t + opts.max_sec).min(total_sec);
        let end = match peaks {
            Some(p) => snap_to_quietest(p, total_sec, target_end, max_end),
            None => target_end,
        };
        chunks.push(Chunk {
            start_sec: t,
            end_sec: end,
        });
        t = end;
    }

    // Merge a too-small trailing chunk into the previous one.
    if let (Some(last), Some(prev)) = (chunks.pop(), chunks.last().cloned()) {
        let last_dur = last.end_sec - last.start_sec;
        if last_dur < opts.min_sec && !chunks.is_empty() {
            // Extend prev.
            let merged_end = last.end_sec;
            let last_idx = chunks.len() - 1;
            chunks[last_idx].end_sec = merged_end;
        } else {
            chunks.push(last);
        }
        // Ensure prev binding is unused warning suppressed.
        let _ = prev;
    }

    chunks
}

fn snap_to_quietest(
    peaks: &WaveformPeaks,
    total_sec: f64,
    target_end: f64,
    max_end: f64,
) -> f64 {
    if peaks.peaks.is_empty() || total_sec <= 0.0 {
        return target_end;
    }
    let n = peaks.n_buckets as f64;
    let target_idx = ((target_end / total_sec) * n) as usize;
    let max_idx = ((max_end / total_sec) * n) as usize;

    let mut best_idx = target_idx.min(peaks.n_buckets as usize - 1);
    let mut best_amp = f32::INFINITY;
    let lo = target_idx.saturating_sub(20);
    let hi = max_idx.min(peaks.n_buckets as usize - 1);
    for i in lo..=hi {
        let min = peaks.peaks.get(i * 2).copied().unwrap_or(0.0);
        let max = peaks.peaks.get(i * 2 + 1).copied().unwrap_or(0.0);
        let amp = (max.abs() + min.abs()) * 0.5;
        if amp < best_amp {
            best_amp = amp;
            best_idx = i;
        }
    }
    let snapped = (best_idx as f64 / n) * total_sec;
    snapped.clamp(target_end - 5.0, max_end)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn short_audio_yields_one_chunk() {
        let chunks = chunk_audio(15.0, None, &ChunkOptions::default());
        assert_eq!(chunks, vec![Chunk { start_sec: 0.0, end_sec: 15.0 }]);
    }

    #[test]
    fn long_audio_chunked_at_target() {
        let chunks = chunk_audio(80.0, None, &ChunkOptions::default());
        assert!(chunks.len() >= 3);
        assert!(chunks
            .windows(2)
            .all(|w| (w[1].start_sec - w[0].end_sec).abs() < 1e-6));
        let last = chunks.last().unwrap();
        assert!((last.end_sec - 80.0).abs() < 1e-6);
    }

    #[test]
    fn no_chunk_smaller_than_min() {
        let opts = ChunkOptions {
            target_sec: 25.0,
            max_sec: 30.0,
            min_sec: 5.0,
        };
        let chunks = chunk_audio(52.0, None, &opts);
        assert!(chunks.iter().all(|c| c.end_sec - c.start_sec >= 5.0 - 1e-6));
    }
}
