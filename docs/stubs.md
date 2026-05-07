# Stub register (the Phase B punch list)

Every "wired but not implemented" site in the workspace, with file:line,
what real implementation is needed, and a rough effort estimate. Treat
this as the canonical TODO list. When you finish a stub, delete its row
and turn the corresponding row in
[`docs/production-readiness.md`](production-readiness.md) green.

Each entry has:

- a short id we can reference in PR titles and CI job names,
- the file:line of the seam,
- what's there now,
- what real implementation is required,
- which CI job will prove it green,
- effort estimate (S = day, M = week, L = month, XL = several months).

## Engine

### S-DOC-001 — `slop-sync::doc::apply_op` only projects `SetProjectSettings`
- **Location**: [`crates/slop-sync/src/doc.rs:78`](../crates/slop-sync/src/doc.rs)
- **Now**: `if let OpKind::SetProjectSettings(p) = ...` projects `fps` and `sample_rate` into the Automerge doc; every other `OpKind` is captured only in the audit log.
- **Need**: project every `OpKind` variant into structured Automerge fields so two clients editing different `OpKind`s converge via CRDT semantics, not just append-log replay.
- **CI gate**: a new `integration-sync-concurrent` job that spins up two `TimelineDoc`s, applies disjoint `OpKind` types on each, exchanges sync messages, and asserts both reconstruct identically.
- **Effort**: M.

### S-RENDER-001 — Cross-dissolve treated as a hard cut
- **Location**: [`crates/slop-render/src/compiler.rs:103`](../crates/slop-render/src/compiler.rs) (the `EffectKind::CrossDissolve` arm in the per-clip-effect loop)
- **Now**: comment says "Approximated: overlap with neighbor handled in future revision. V1 ignores cross-dissolve at render time and treats it as a hard cut."
- **Need**: emit overlapping `xfade` filtergraph between adjacent clips on the same track when one carries `CrossDissolve`. The `slop-transitions` catalog already has the right xfade names; integration is in the render compiler.
- **CI gate**: render a fixture timeline with a cross-dissolve, decode N frames around the cut, assert frame at midpoint is a blend (not equal to either source frame).
- **Effort**: M.

### S-COLOR-001 — CDL via `eq=brightness/gamma` approximation
- **Location**: [`crates/slop-color/src/ffmpeg.rs:38`](../crates/slop-color/src/ffmpeg.rs)
- **Now**: source comment: "For a full SOTA pipeline this would emit a synthesized 3D LUT and pass it through `lut3d` for stability across hosts."
- **Need**: when emitting CDL, generate a synthetic Identity LUT, apply the CDL math to it in software, and emit a `lut3d=` filtergraph. The `slop-color::cdl::apply_cdl_pixel` function already exists and is unit-tested; reuse it to build the table.
- **CI gate**: a `cdl-roundtrip` test that bakes a known CDL into a 17^3 LUT, applies it to a known input image via ffmpeg, and asserts pixel values match the reference `apply_cdl_pixel` output within tolerance.
- **Effort**: S.

### S-WEB-001 — `apps/web::App::replay()` does nothing
- **Location**: [`apps/web/src/App.tsx:96`](../apps/web/src/App.tsx) (the `replay()` function)
- **Now**: `void op` — the loop iterates the audit log entries and discards them.
- **Need**: project entries into the SlopTimeline. Cleanest path is to compile `slop-core` to WASM and call its replay from JS. Until then, a TS reimplementation of the reducer for the V1 op kinds.
- **CI gate**: a Playwright test that loads a fixture project from a sync-server and asserts the rendered timeline canvas shows the expected clip count.
- **Effort**: M (TS reimpl) or L (WASM via `wasm-pack`).

## AI

### S-ASR-001 — `whisper-cpp` backend doesn't run inference
- **Location**: [`crates/slop-asr/src/backend/whisper_cpp.rs:60`](../crates/slop-asr/src/backend/whisper_cpp.rs) (`run_whisper`)
- **Now**: under the `whisper-cpp` feature, calls into `whisper-rs`'s `WhisperContext` with hardcoded sampling params; works in principle, but the chunker passes do not validate against ground-truth transcripts.
- **Need**: integration test against a downloaded `ggml-tiny.en.bin`, transcribe a 30-second fixture WAV, assert WER < 10% against the checked-in ground-truth caption file.
- **CI gate**: `integration-whisper` job in [`ci.yml`](../.github/workflows/ci.yml).
- **Effort**: M.

### S-DIAR-001 — pyannote ONNX forward passes are stubs
- **Location**: [`crates/slop-asr/src/diarize.rs:243` and `:259`](../crates/slop-asr/src/diarize.rs) (`run_segmentation`, `run_embedding`)
- **Now**: both return `Err("ort segmentation forward not yet implemented in this build")` regardless of feature flag.
- **Need**: real ONNX inference. Inputs: 16 kHz mono PCM, sliding-window 10s for segmentation, 5s chunks for embedding. Outputs: per-frame 4-class softmax for segmentation, 192-dim L2-normalized vector for embedding. Use the `ndarray` integration in `ort 2.x` for tensor IO.
- **CI gate**: `integration-onnx-pyannote` job that downloads pyannote ONNX models, runs Diarizer::diarize on a 2-speaker fixture, asserts DER > 0.7 (i.e. correct labels at least 70% of the time on a known reference).
- **Effort**: L.

### S-YOLO-001 — YOLOv11 detection path returns empty Vec
- **Location**: [`crates/slop-reframe/src/yolo.rs:51`](../crates/slop-reframe/src/yolo.rs) (`detect`)
- **Now**: under `--features ort`, returns `Ok(Vec::new())` regardless of input.
- **Need**: real ONNX inference: load `yolo11n.onnx`, letterbox-preprocess input to 640x640, run forward pass, decode 1x84x8400 output (4 box + 80 class scores), apply NMS via `crate::yolo::nms`.
- **CI gate**: `integration-yolo` job that downloads `yolo11n.onnx`, runs detection on a fixture image known to contain a person, asserts a person bbox is found within 10px of the ground-truth coordinates.
- **Effort**: M.

### S-GENAV-001 — ComfyUI provider not exercised against a real server
- **Location**: [`crates/slop-genav/src/broll.rs:84`](../crates/slop-genav/src/broll.rs) (`ComfyUiProvider::generate`)
- **Now**: code is structurally complete; `bind_node`, `urlencode`, the polling loop, and the file-write are all unit-testable and tested. But there is no integration test that posts a real workflow to a running ComfyUI server.
- **Need**: a docker-compose-based integration test that spins up a ComfyUI container with a tiny workflow (no real model weights needed; can use a 1-frame placeholder) and round-trips a single B-roll generation.
- **CI gate**: `integration-comfyui` job. Allowed to fail until the docker setup is stable.
- **Effort**: M.

### S-VOICE-001 — XTTS-v2 provider not exercised
- **Location**: [`crates/slop-genav/src/voice.rs:97`](../crates/slop-genav/src/voice.rs) (`XttsProvider::synthesize`)
- **Now**: HTTP shape is correct; consent ledger gate works. No integration test.
- **Need**: docker-compose Coqui-TTS server, synthesize a single line, assert the resulting WAV decodes to ~the expected duration.
- **CI gate**: `integration-voice` job.
- **Effort**: M.

## Platform

### S-CLI-001 — `slop` CLI no end-to-end test
- **Location**: [`apps/cli/src/main.rs`](../apps/cli/src/main.rs)
- **Now**: compiles; `cmd_ingest`, `cmd_plan`, `cmd_render`, `cmd_export` are real but not exercised.
- **Need**: a fixture project + a CI job that runs `slop ingest`, `slop render`, `slop export --target otio` and asserts the artifacts exist and parse.
- **CI gate**: `integration-cli`.
- **Effort**: S.

### S-PY-001 — PyO3 wheel never built
- **Location**: [`bindings/python/`](../bindings/python/)
- **Now**: compiles via `cargo check`. Never built into a wheel via `maturin build`.
- **Need**: CI job that runs `maturin build`, installs the wheel, asserts `import slop_py; slop_py.PyTimeline()` works.
- **CI gate**: `integration-bindings-py`.
- **Effort**: S.

### S-NODE-001 — napi-rs binary never built
- **Location**: [`bindings/node/`](../bindings/node/)
- **Now**: compiles via `cargo check`. Never built into a `.node` artifact via `napi build`.
- **Need**: CI job that runs `napi build --release` and `node -e "require('@slop/node')"`.
- **CI gate**: `integration-bindings-node`.
- **Effort**: S.

### S-PLUGIN-001 — Wasmtime plugin host has no example plugin
- **Location**: [`crates/slop-plugin/src/host.rs`](../crates/slop-plugin/src/host.rs)
- **Now**: host can load a component if you have one; no fixture component exists.
- **Need**: build a minimal `examples/plugins/hello-effect/` Rust crate that compiles to `wasm32-wasip2` component, write a host integration test that loads it and asserts a tool call round-trips.
- **CI gate**: `integration-plugin`.
- **Effort**: M.

### S-TAURI-001 — Tauri build + sign + notarize never run
- **Location**: [`apps/desktop/src-tauri/tauri.conf.json`](../apps/desktop/src-tauri/tauri.conf.json), [`.github/workflows/release.yml`](../.github/workflows/release.yml)
- **Now**: `cargo check -p slop-desktop` works on macOS. Full `tauri build` + bundle generation never run.
- **Need**: CI job that runs `tauri build` on macOS + Linux + Windows runners, with notarization secrets present.
- **CI gate**: `integration-tauri`.
- **Effort**: L (signing certs + notarization workflow are non-trivial).

### S-IPC-001 — V1.5+ crates not exposed via Tauri IPC
- **Location**: [`apps/desktop/src-tauri/src/commands.rs`](../apps/desktop/src-tauri/src/commands.rs)
- **Now**: 18 IPC commands cover V1 (assets, transcript, scenes, plan, pin/unpin, render, export OTIO). V1.5/V2.0/V2.5/V3.0 crates (multicam, color, mixer, transitions, captions, vision, agent, genav, reframe, sync, plugin) are not callable from the frontend.
- **Need**: one tranche of IPC commands per release theme, each with a Vitest mock and a Tauri integration smoke test.
- **CI gate**: `tauri-ipc-smoke` job per release.
- **Effort**: M-L per release.

## Verification rigor

### S-VERIF-001 — No property-based tests
- **Location**: workspace-wide, target is `slop-core`
- **Now**: 35 unit tests in `slop-core` cover the happy paths and a handful of edge cases.
- **Need**: `proptest` strategies for `Timeline`, `Op`, `Plan`. Run reducer apply/replay across random op sequences; run validator/repair across random invalid documents.
- **CI gate**: a `proptest` job on `cargo test --features proptest`.
- **Effort**: M.

### S-VERIF-002 — No mutation testing
- **Location**: workspace-wide
- **Now**: nothing.
- **Need**: `cargo-mutants` configured against `slop-core` first (then expand). Target ≥ 80% mutation kill rate.
- **CI gate**: weekly `mutants` job on schedule.
- **Effort**: M (initial), then ongoing.

### S-VERIF-003 — No fuzz testing
- **Location**: workspace-wide
- **Now**: nothing.
- **Need**: `cargo-fuzz` targets for the JSON Schema validator, OTIO writer, FCP7/FCPXML/Kdenlive adapters, and the `.cube` parser.
- **CI gate**: weekly `fuzz` job, 60 minutes per target.
- **Effort**: M.

### S-VERIF-004 — A11y audit not run
- **Location**: [`scripts/a11y-audit.mjs`](../scripts/a11y-audit.mjs), [`apps/web/`](../apps/web/)
- **Now**: a11y audit script committed; never run against a real server.
- **Need**: CI job that starts the web companion and runs Playwright + axe-core, asserts zero WCAG 2.2 AA violations.
- **CI gate**: `integration-a11y`.
- **Effort**: M (the violations themselves will take time to fix).

## Closed (was a stub, now real)

(Empty for now. As Phase B sessions land real implementations, move rows
from above to here with the PR/CI link that proved them.)
