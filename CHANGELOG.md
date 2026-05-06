# Changelog

All notable changes to this project will be documented in this file.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and the project adheres to [Semantic Versioning](https://semver.org/).

## [Unreleased]

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
