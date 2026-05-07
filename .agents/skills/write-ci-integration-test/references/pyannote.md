# integration-onnx-pyannote

Closes `S-DIAR-001` in [docs/stubs.md](../../../../docs/stubs.md).

## Fixture

- 60-second 2-speaker WAV at `crates/slop-asr/tests/fixtures/pyannote/conv.wav`.
- Ground-truth RTTM at `crates/slop-asr/tests/fixtures/pyannote/conv.rttm` (start, duration, speaker label per turn).

## Job body

```yaml
- name: Install build deps
  run: sudo apt-get update && sudo apt-get install -y cmake clang ffmpeg
- name: Cache pyannote ONNX models
  id: model-cache
  uses: actions/cache@v4
  with:
    path: ~/.cache/slop/asr
    key: pyannote-onnx-3.0-v1
- name: Download pyannote models
  if: steps.model-cache.outputs.cache-hit != 'true'
  run: |
    mkdir -p ~/.cache/slop/asr
    curl -L -o ~/.cache/slop/asr/pyannote_segmentation_3_0.onnx \
      https://github.com/pyannote/pyannote-audio/releases/download/onnx-3.0/segmentation-3.0.onnx
    curl -L -o ~/.cache/slop/asr/pyannote_embedding.onnx \
      https://github.com/pyannote/pyannote-audio/releases/download/onnx-3.0/embedding.onnx
    sha256sum ~/.cache/slop/asr/*.onnx
- name: Run diarization integration suite
  run: cargo test -p slop-asr --features ort --test diarize_e2e -- --nocapture
```

## Test (in `crates/slop-asr/tests/diarize_e2e.rs`)

```rust
#[test]
#[cfg_attr(not(feature = "ort"), ignore)]
fn diarize_fixture_meets_der_threshold() {
    let cfg = DiarConfig::new(format!("{}/.cache/slop/asr", std::env::var("HOME").unwrap()));
    let diarizer = Diarizer::new(cfg);
    let pcm = read_wav_to_f32_mono_16k("tests/fixtures/pyannote/conv.wav").unwrap();
    let spans = diarizer.diarize(&pcm, 16_000).unwrap();
    let truth = parse_rttm("tests/fixtures/pyannote/conv.rttm");
    let der = diarization_error_rate(&spans, &truth);
    assert!(der < 0.30, "DER too high: {der:.3}");
}
```

DER = `(missed_speech + false_alarm + speaker_confusion) / total_speech`. The accepted SOTA threshold is ~0.10 on AMI; we set 0.30 for the small fixture and the tiny embedding model.

## Promotion criteria

- 3+ green runs on `main`.
- DER assertion holds with the pinned models.
- Then drop `continue-on-error`, mark `S-DIAR-001` green.
