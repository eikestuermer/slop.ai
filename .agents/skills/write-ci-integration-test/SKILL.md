---
name: write-ci-integration-test
description: >-
  Replace one of the Tier-2 placeholder bodies in
  .github/workflows/ci.yml with a real integration test. Use when the user
  asks to "wire integration test for X", "make the integration-X job real",
  or as the second half of a turn-stub-green session. Each Tier-2 job
  follows a common shape: install deps, download fixture, run real binary,
  assert quality metric. The references/ subfolder has per-integration
  templates (whisper, pyannote, yolo, ffmpeg-render, otio-roundtrip, sync,
  plugin, tauri, bindings-py, bindings-node, a11y).
---

# Write a CI integration test

The Tier-2 jobs in [.github/workflows/ci.yml](.github/workflows/ci.yml) are wired and allowed-to-fail today; their bodies are placeholder `echo "TODO" ; exit 0`. This skill replaces one of them with a real test that:

1. Installs the platform deps it needs.
2. Downloads or generates a deterministic fixture.
3. Runs the real binary or service against the fixture.
4. Asserts a quality metric, not just exit code 0.

## Common shape

Every integration test in `ci.yml` follows this skeleton:

```yaml
integration-<name>:
  name: Integration - <description> [allowed-to-fail]
  runs-on: ubuntu-latest        # or matrix
  continue-on-error: true       # remove only after the job is consistently green
  steps:
    - uses: actions/checkout@v4
    - uses: dtolnay/rust-toolchain@stable
    - uses: Swatinem/rust-cache@v2
    - name: Install deps
      run: sudo apt-get update && sudo apt-get install -y <list>
    - name: Download fixture
      run: |
        mkdir -p tests/fixtures/<name>
        curl -L -o tests/fixtures/<name>/<file> https://<url>
    - name: Run integration suite
      run: |
        # Real test invocation here.
        cargo test -p <crate> --test <name>_integration -- --nocapture
```

## Promoting from allowed-to-fail to gating

When the job has been consistently green on `main` for at least 3 consecutive runs:

1. Remove `continue-on-error: true`.
2. Update [docs/production-readiness.md](docs/production-readiness.md) row to green with the CI run citation.
3. Move the matching `S-` entry in [docs/stubs.md](docs/stubs.md) to "Closed".

## Per-integration templates

See the `references/` subfolder of this skill:

- [references/whisper.md](references/whisper.md) — `integration-whisper`: download `ggml-tiny.en.bin`, transcribe a 30-second fixture WAV, assert WER < 10% against ground truth.
- [references/pyannote.md](references/pyannote.md) — `integration-onnx-pyannote`: download pyannote ONNX models, run `Diarizer::diarize` on a 2-speaker fixture, assert > 0.7 DER.
- [references/yolo.md](references/yolo.md) — `integration-yolo`: download `yolo11n.onnx`, run `YoloDetector::detect` on a fixture image, assert person bbox within 10px of ground-truth.
- [references/ffmpeg-render.md](references/ffmpeg-render.md) — `integration-ffmpeg-render`: render a fixture project, assert output MP4 decodes, assert checksummed golden frame at known timestamp.
- [references/otio-roundtrip.md](references/otio-roundtrip.md) — `integration-otio-roundtrip`: emit OTIO + FCP7 + FCPXML + Kdenlive on a fixture project, validate via `xmllint` + `otioconvert`.
- [references/sync.md](references/sync.md) — `integration-sync`: spin up `slop-sync-server`, connect two `TimelineDoc` clients, apply concurrent edits, assert convergence.
- [references/plugin.md](references/plugin.md) — `integration-plugin`: build a fixture WASI component (the `examples/plugins/hello-effect` plan from `S-PLUGIN-001`), load it into PluginHost, assert tool call round-trips.
- [references/tauri.md](references/tauri.md) — `integration-tauri`: full `tauri build` on macOS+Linux+Windows. Notarization wired via repo secrets when present.
- [references/bindings-py.md](references/bindings-py.md) — `integration-bindings-py`: `maturin build` + install + `import slop_py; PyTimeline()`.
- [references/bindings-node.md](references/bindings-node.md) — `integration-bindings-node`: `napi build --release` + `require()` smoke.
- [references/a11y.md](references/a11y.md) — `integration-a11y`: Playwright + axe-core against a running web companion; zero WCAG 2.2 AA violations.

## Anti-patterns

- **Don't replace `echo "TODO"` with `echo "implemented"`**. The new body must actually exercise the implementation. If you can't run the real workload yet (e.g. fixture media isn't ready), that's the work to do first.
- **Don't ship the real test as `continue-on-error: true` and then immediately mark the row green**. The promotion-to-gating step is mandatory; until then the row is yellow at best.
- **Don't pin model files to public CDNs without a checksum**. Verify SHA-256 before running; a swapped fixture can cause spurious failures or mask regressions.
- **Don't assert "exit code 0"**. Always assert a numerical quality metric. "It ran" is not what we're proving.
