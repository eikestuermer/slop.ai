# Changelog

All notable changes to this project will be documented in this file.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and the project adheres to [Semantic Versioning](https://semver.org/).

## [Unreleased] (post-V1 SOTA scaffolds for the 2-year roadmap)

### V1.0 polish
- `slop-asr` ports the whisper.cpp backend onto the `whisper-rs` crate
  (feature-gated). Real model download manager with SHA-256 verification
  and atomic `.partial` swap. New `download_with_checksum` helper shared
  with the diarization model loader.
- Diarization replaced with a pyannote-audio 3.0 pipeline scaffold backed
  by ONNX Runtime via `ort` (feature-gated) — segmentation + x-vector +
  agglomerative clustering.
- Real Tauri icon set: hand-authored SVG plus a `sharp`-based rasterizer
  that emits the full PNG/ICNS/ICO matrix Tauri's bundler expects.
- Release workflow gains macOS notarization + Windows code signing wiring
  (Apple Developer ID + Azure Trusted Signing).

### V1.5 "Editor's edge"
- New `timeline.v2.json` schema: compound clips, multicam groups, J/L
  cut offsets (audio_offset_sec / video_offset_sec), speed curves with
  keyframes, effect graph (EffectNode with typed params + keyframes),
  styled captions, transitions, project-level mixer + color pipeline.
- v1 -> v2 migration in `crates/slop-core/src/migrations/v1_to_v2.rs`.
- New crates:
  - `slop-multicam`: FFT-based audio cross-correlation (rustfft +
    realfft) for angle sync.
  - `slop-color`: ASC CDL + Iridas `.cube` 3D LUT loader with trilinear
    interpolation, FFmpeg `lut3d` + `colorchannelmixer` filtergraph
    emitter, scope generators (waveform / vectorscope / parade /
    histogram).
  - `slop-mixer`: ITU-R BS.1770-4 LUFS metering with K-weighting biquads,
    LRA computation, `loudnorm` two-pass filtergraph emitter.
  - `slop-transitions`: 15-entry `xfade` transition catalog matching V2
    schema enum.
  - `slop-captions`: SRT + WebVTT + ASS writers driven by the V2 schema's
    CaptionStyle.

### V2.0 "AI studio"
- New crates:
  - `slop-vision`: multi-modal prompt-pack assembly. Frame tiling at
    448x448 to match Qwen2.5-VL / Llava-OneVision native resolution;
    visual budget planner.
  - `slop-agent`: agentic edit loop on the OpenAI tools protocol. Tool
    registry with JSON Schema validation, self-critic pass, multi-loop
    iteration with critique feedback.
  - `slop-genav`: ComfyUI provider for B-roll generation (Wan2.1 default
    workflow); XTTS-v2 voice provider with consent ledger; OpenAI-compat
    translator + DubPipeline.
  - `slop-reframe`: Kalman-smoothed crop-track solver for vertical/square
    delivery; YOLOv11-nano detector wiring (ONNX Runtime, feature-gated);
    NMS implementation.

### V2.5 "Together"
- New crate `slop-sync`: Automerge-backed timeline document, the
  Automerge sync protocol over WebSocket, ed25519 identity (no central
  account server), per-project ACL with role hierarchy
  (Viewer/Reviewer/Editor/Owner).
- New app `apps/sync-server`: Axum + sled service speaking the sync
  protocol. Dockerfile + systemd unit ship in the repo so self-hosting
  is one command.
- New app `apps/web`: read-only PWA companion for project review.
  Connects to a sync server, displays the timeline canvas via the shared
  `@slop/ui-timeline` package, registers a service worker for offline
  use.

### V3.0 "Ecosystem"
- New crate `slop-plugin`: WASI Component Model 0.2 plugin host on
  Wasmtime 27. Capability-based security (Effects, ScoringFeatures,
  Exporters, PromptPackStyles, FsRead, FsWrite, Network), Sigstore
  signature verification, decentralized registry model.
- New app `apps/cli`: full `slop` headless CLI (ingest / plan / render /
  export) for CI and batch use.
- Bindings: Python (PyO3 + maturin) and Node (napi-rs) packages exposing
  the timeline + render + OTIO surface.
- Governance: explicit steering committee model in `docs/governance.md`,
  RFC template at `docs/rfcs/0000-template.md`, security policy in
  `SECURITY.md`, maintainers list, funding doc.

### V3.5 "Polish"
- i18n: i18next + ICU MessageFormat 2 in the desktop frontend with five
  full locales (en, de, es, fr, ja). Plurals, gender, select expressions
  all handled by ICU.
- Accessibility: axe-core + Playwright audit script enforcing WCAG 2.2
  AA in CI (`scripts/a11y-audit.mjs`).
- Performance: criterion benchmarks for the reducer hot path
  (100/1k/10k clip apply).
- LTS: `docs/lts.md` declares a 2-year support window per major release;
  release-branch + tagging convention.

## [0.1.0] - V1 initial scaffold

### Added

- Initial monorepo skeleton.
- `slop-core`: canonical `Timeline` schema + reversible `Op` log + reducer +
  validator + repair pass.
- `slop-media`: ffprobe / proxy / thumb-strip / waveform-peaks wrappers.
- `slop-asr`: pluggable ASR backend trait, pure-Rust placeholder backend,
  whisper.cpp scaffold (feature-gated), model manager, silence-aware
  chunker.
- `slop-scenes`: Rust port of PySceneDetect's `ContentDetector` and an
  `AdaptiveDetector`.
- `slop-score`: rule-based candidate moment builder + scorer + prompt-pack
  builder.
- `slop-planner`: OpenAI-compatible chat-completions client with JSON Schema
  constrained outputs, validator + deterministic repair pass.
- `slop-render`: FFmpeg filtergraph compiler + runner, with optional caption
  burn-in and SRT sidecar.
- `slop-otio`: pure-Rust OTIO subset writer, plus FCP7 XML (Premiere),
  FCPXML (Resolve), and Kdenlive (MLT XML) adapters.
- Tauri 2.x desktop app with React 18 + TypeScript shell, timeline canvas,
  inspector, prompt bar, and BYO endpoint configuration UI.
- Privacy mode (localhost-only endpoint enforcement, sentinel file in
  project root).
- CI matrix: macOS, Linux, Windows; rust + frontend + schema-validation +
  cargo-deny license audit.
- Release workflow that builds signed-ready bundles for all three desktop
  platforms.
- Documentation: architecture, schema, export fidelity, license posture,
  non-goals, BYO endpoint, contributing.
