# integration-whisper

Closes `S-ASR-001` in [docs/stubs.md](../../../../docs/stubs.md).

## Fixture

- 30-second WAV (mono, 16 kHz) at `crates/slop-asr/tests/fixtures/whisper/sample.wav`.
- Ground-truth transcript at `crates/slop-asr/tests/fixtures/whisper/sample.txt`.
- Source: a snippet of LibriSpeech / Common Voice / CC-BY public-domain audio.
- Pin a sha256 in `crates/slop-asr/tests/fixtures/whisper/SHA256SUMS`.

## Job body

```yaml
- name: Install build deps
  run: sudo apt-get update && sudo apt-get install -y cmake clang ffmpeg
- name: Cache whisper.cpp model
  id: model-cache
  uses: actions/cache@v4
  with:
    path: ~/.cache/slop/asr/ggml-tiny.en.bin
    key: whisper-tiny-en-v1
- name: Download whisper.cpp model (tiny.en, 75MB)
  if: steps.model-cache.outputs.cache-hit != 'true'
  run: |
    mkdir -p ~/.cache/slop/asr
    curl -L -o ~/.cache/slop/asr/ggml-tiny.en.bin \
      https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-tiny.en.bin
    echo "921e4cf8686fdd993dcd081a5da5b6c365bfde1162e72b08d75ac75289920b1f  ~/.cache/slop/asr/ggml-tiny.en.bin" \
      | sha256sum -c
- name: Verify fixture
  run: cd crates/slop-asr/tests/fixtures/whisper && sha256sum -c SHA256SUMS
- name: Run whisper integration suite
  run: cargo test -p slop-asr --features whisper-cpp --test whisper_e2e -- --nocapture
```

## Test (in `crates/slop-asr/tests/whisper_e2e.rs`)

```rust
#[tokio::test]
#[cfg_attr(not(feature = "whisper-cpp"), ignore)]
async fn transcribes_fixture_with_low_wer() {
    let model_path = std::env::var("HOME").map(|h| format!("{h}/.cache/slop/asr/ggml-tiny.en.bin")).unwrap();
    let backend = WhisperCppBackend::new(model_path);
    let job = AsrJob {
        asset_id: "fixture".into(),
        input: "tests/fixtures/whisper/sample.wav".into(),
        duration_sec: 30.0,
    };
    let transcript = backend.transcribe(job, &AsrOptions::default()).await.unwrap();
    let got: String = transcript.segments.iter().map(|s| s.text.as_str()).collect::<Vec<_>>().join(" ");
    let expected = std::fs::read_to_string("tests/fixtures/whisper/sample.txt").unwrap();
    let wer = wer_score(&got, &expected);
    assert!(wer < 0.10, "WER too high: got={got!r}, expected={expected!r}, wer={wer:.3}");
}

fn wer_score(hypothesis: &str, reference: &str) -> f32 {
    // Levenshtein-based WER on whitespace-tokenized lowercased strings.
    // ... ~30 lines, can borrow from a small WER crate or implement directly.
}
```

## Promotion criteria

- `integration-whisper` green on `main` for 3+ runs.
- WER assertion holds.
- Then drop `continue-on-error: true`, mark `S-ASR-001` row green.
