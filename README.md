# Slop AI

> Local-first, prompt-driven rough-cut video editor. Open source.

Slop AI generates rough-cut timelines from your raw footage and a one-line brief.
You review, pin, ban, and refine. Export to MP4, OTIO, or hand off to Premiere,
Resolve, or Kdenlive.

It runs entirely on your machine. No cloud account required. Bring your own
OpenAI-compatible LLM endpoint (Ollama, llama.cpp server, LM Studio, or a hosted
provider) for the planner.

## Status

Pre-release. The project is being built in the open. See [docs/architecture.md](docs/architecture.md)
for the design and [docs/non-goals.md](docs/non-goals.md) for what V1 deliberately does not do.

## Architectural pillars

1. **Two-layer state**. App-state JSON + reversible command log (`ops.jsonl`) is
   the source of truth. OTIO is a *derived* interchange artifact, never the
   canonical store.
2. **Candidate-set planning**. The LLM only ever selects from precomputed
   candidate moments. It plans, it never discovers raw media.
3. **Validate then apply**. Every LLM output passes through a schema validator
   and a deterministic repair pass before it touches the timeline.
4. **Deterministic render**. Previews and exports compile through FFmpeg
   filtergraphs from the validated app-state, not from LLM output.
5. **Rough-cut promise only**. V1 promises cuts, trims, markers, captions, and
   simple speed. It explicitly does not promise round-tripping transitions,
   effect stacks, or speed ramps to professional NLEs.

## Repo layout

```
slop.ai/
  apps/desktop/         Tauri 2.x app (forked from OpenReelio shell)
    src/                React + TS frontend
    src-tauri/          Rust host: commands, IPC, app-state
  crates/
    slop-core/          Timeline schema + ops log + validator
    slop-media/         FFmpeg/ffprobe wrappers, proxies, thumbs, waveforms
    slop-asr/           whisper.cpp bindings + chunking
    slop-scenes/        PySceneDetect ContentDetector port
    slop-score/         Candidate moment builder + feature scoring
    slop-planner/       OpenAI-compatible client + structured output + repair
    slop-otio/          OTIO subset writer + adapter targets
    slop-render/        FFmpeg filtergraph compiler
  packages/
    schemas/            JSON Schema files (single source of truth)
    ui-timeline/        Reusable timeline canvas React components
  examples/             Sample fixture media + golden plans
  docs/                 Architecture, schema, export-fidelity, contributing
```

## Quick start

This is a Cargo workspace plus a pnpm-managed frontend.

```bash
# Build all Rust crates and run unit tests
cargo test

# Build the desktop shell (needs pnpm and Tauri prerequisites)
pnpm install
pnpm --filter @slop/desktop tauri dev
```

See [docs/contributing.md](docs/contributing.md) for the full development guide
including platform-specific Tauri prerequisites and FFmpeg setup.

## License

MIT. See [LICENSE](LICENSE).

The project draws architectural inspiration and a small amount of code from
[OpenReelio](https://github.com/openreelio/openreelio) (also MIT). FFmpeg is
used as an LGPL-2.1+ dependency; we ship LGPL-only builds and explicitly do
*not* enable `--enable-gpl`. See [docs/license-posture.md](docs/license-posture.md).
