# Slop AI

> Local-first, prompt-driven rough-cut video editor. Open source.

Slop AI generates rough-cut timelines from your raw footage and a one-line
brief. You review, pin, ban, and refine. Export to MP4, OTIO, or hand off
to Premiere, Resolve, or Kdenlive.

It runs entirely on your machine. No cloud account required. Bring your own
OpenAI-compatible LLM endpoint (Ollama, llama.cpp server, LM Studio, or a
hosted provider) for the planner.

## Status

Pre-release. Built in the open. The 2-year roadmap, the production-readiness
status board, and the explicit punch list of what's still wired-but-not-real
are linked under "How to contribute" below.

## Want to help? Start here

- **First time?** Read [docs/onboarding.md](docs/onboarding.md). It's a
  10-minute walkthrough from `git clone` to your first green CI run.
- **Want a task?** Pick one from [docs/good-first-issues.md](docs/good-first-issues.md).
  Stubs are tagged `S-XXX-NNN` and ranked by effort (S/M/L/XL).
- **The full punch list** is in [docs/stubs.md](docs/stubs.md). Every entry
  cites the file:line of the seam, what's needed, and the CI job that gates
  it. The matching status board is [docs/production-readiness.md](docs/production-readiness.md).
- **House rules** for code conventions, schema discipline, and the four
  workstreams (Engine / AI / Platform / Community) live in
  [AGENTS.md](AGENTS.md). Read it before making changes.
- **Contributing process**, prerequisites, lint and test commands, PR
  conventions: [CONTRIBUTING.md](CONTRIBUTING.md).
- **Governance**, RFC process, decision-making: [docs/governance.md](docs/governance.md).
- **Code of conduct**: [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md) (Contributor
  Covenant 2.1).
- **Security disclosure**: [SECURITY.md](SECURITY.md). Coordinated, 90-day
  default embargo.
- **Questions or design discussions**: GitHub Discussions on this repo (once
  enabled by a maintainer; see [docs/onboarding.md](docs/onboarding.md) for
  alternatives until then).

## Architectural pillars

1. **Two-layer state.** App-state JSON + reversible command log (`ops.jsonl`)
   is the source of truth. OTIO is a *derived* interchange artifact, never
   the canonical store.
2. **Candidate-set planning.** The LLM only ever selects from precomputed
   candidate moments. It plans; it never discovers raw media.
3. **Validate then apply.** Every LLM output passes through a schema
   validator and a deterministic repair pass before it touches the timeline.
4. **Deterministic render.** Previews and exports compile through FFmpeg
   filtergraphs from the validated app-state, not from LLM output.
5. **Rough-cut promise only.** V1 promises cuts, trims, markers, captions,
   and simple speed. It explicitly does *not* promise round-tripping
   transitions, effect stacks, or speed ramps to professional NLEs.

See [docs/architecture.md](docs/architecture.md) for the long form and
[docs/non-goals.md](docs/non-goals.md) for what V1 deliberately does not do.

## Repo layout

```
slop.ai/
  apps/
    desktop/        Tauri 2.x desktop shell (React + TS frontend, Rust host)
    sync-server/    Self-hosted CRDT sync server (Axum + sled)
    cli/            Headless `slop` CLI (ingest / plan / render / export)
    web/            Read-only PWA review companion
  crates/
    slop-core/      Timeline schema + ops log + reducer + validator + repair
    slop-media/     FFmpeg/ffprobe wrappers (proxies, thumbs, waveforms)
    slop-asr/       whisper.cpp + pyannote ONNX scaffolds
    slop-scenes/    PySceneDetect ContentDetector + AdaptiveDetector port
    slop-score/     Candidate moment builder + scorer
    slop-planner/   OpenAI-compatible LLM client + JSON Schema constraint
    slop-otio/      OTIO subset writer + Premiere/Resolve/Kdenlive adapters
    slop-render/    FFmpeg filtergraph compiler + runner
    slop-multicam/  FFT cross-correlation audio sync
    slop-color/     ASC CDL + Iridas .cube LUT loader + scope generators
    slop-mixer/     ITU-R BS.1770-4 LUFS metering
    slop-transitions/  xfade transition catalog
    slop-captions/  SRT / WebVTT / ASS writers
    slop-vision/    Multi-modal frame tile extractor
    slop-agent/     Agentic edit loop (OpenAI tool-call protocol)
    slop-genav/     ComfyUI / XTTS-v2 / dub scaffolds + voice consent ledger
    slop-reframe/   YOLO-driven smart-crop solver
    slop-sync/      Automerge CRDT timeline + WebSocket sync + ed25519 ACL
    slop-plugin/    WASI Component Model 0.2 plugin host (Wasmtime)
  bindings/
    python/         PyO3 + maturin
    node/           napi-rs
  packages/
    schemas/        JSON Schemas + TS codegen (single source of truth)
    ui-timeline/    Reusable timeline canvas React components
  examples/         Sample fixture media + golden plans
  docs/             Architecture, schema, export-fidelity, contributing, ...
  .agents/skills/   Workspace skills for recurring workflows
  .cursor/          Rules, subagents, hooks (workspace-committed)
```

## Quick start

```bash
# Run every Rust test (140+ tests; needs Rust 1.75+).
cargo test --workspace

# Validate JSON Schemas + regenerate TypeScript types.
pnpm install
pnpm --filter @slop/schemas validate
pnpm --filter @slop/schemas codegen

# Run the desktop shell in dev mode (needs Tauri prerequisites + ffmpeg).
pnpm --filter @slop/desktop tauri dev
```

The full setup (platform prerequisites, Tauri toolchain, optional whisper.cpp /
Ollama for end-to-end testing) is in [docs/onboarding.md](docs/onboarding.md).

## License

MIT. See [LICENSE](LICENSE).

The project draws architectural inspiration and a small amount of code from
[OpenReelio](https://github.com/openreelio/openreelio) (also MIT). FFmpeg is
used as an LGPL-2.1+ dependency; we ship LGPL-only builds and explicitly do
*not* enable `--enable-gpl`. See [docs/license-posture.md](docs/license-posture.md).
