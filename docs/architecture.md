# Architecture

Slop AI is a local-first, prompt-driven rough-cut video editor. This document
explains how the major pieces fit together.

## High-level flow

```
ingest  ->  ffprobe + proxies + thumbs + waveforms  (slop-media)
            transcript                              (slop-asr)
            scene boundaries                        (slop-scenes)
            ->  candidate moments                   (slop-score)
            ->  planner LLM (BYO endpoint)          (slop-planner)
            ->  validate + repair                   (slop-core)
            ->  Op log + Timeline                   (slop-core)
            ->  preview MP4                         (slop-render)
            ->  OTIO / FCP7 XML / Resolve / Kdenlive (slop-otio)
```

## The two-layer state model

Everything in Slop AI ultimately resolves to two artefacts:

1. **`timeline.json`** - a single JSON document conforming to
   `packages/schemas/timeline.v1.json`. This is the canonical app-state.
2. **`ops.jsonl`** - an append-only log of reversible commands conforming to
   `packages/schemas/ops.v1.json`. Replaying this log against an empty
   `Timeline::empty()` reconstructs `timeline.json` exactly.

The in-memory `Timeline` struct is a *cache* of these two files. If the cache
is ever lost (crash, version mismatch, bad save), recovery is a function of
the op log alone.

OTIO and the various NLE export formats are *derived* from the canonical
state. They are write-targets, not read-sources.

## Why two layers and not one?

Three reasons:

- **Undo / redo** is free when every mutation has an inverse.
- **AI patching** is sane: the planner emits a small set of `Op`s, those go
  through validation and repair, and only valid ops touch the timeline.
- **Crash recovery** is just log replay; we never need to corrupt-fix a
  binary database.

## Candidate-set planning

The planner LLM is *not* allowed to discover raw media. It receives:

- a project goal (the user's prompt),
- a precomputed candidate list (transcript segments + shot boundaries +
  feature scores), each candidate identified by stable IDs,
- a JSON Schema describing the response shape.

The planner returns JSON. That JSON is validated against
`plan.v1.json`, then validated against the candidate set (do these IDs exist?
are these timestamps within the asset's duration? do these clips overlap on
this track?), then deterministically repaired where possible. Whatever
remains valid is converted into `Op`s.

The model never sees a filtergraph. The model never invents IDs. If the model
returns garbage, the validator surfaces it; we do not silently mutate.

## Process model

Slop AI ships as a single Tauri 2.x application:

- **Frontend** (`apps/desktop/src/`) - React 18 + TypeScript. Renders the
  timeline canvas, the prompt UX, and the inspector. Talks to the host via
  Tauri IPC commands.
- **Rust host** (`apps/desktop/src-tauri/`) - owns project state, the op log,
  job orchestration, and embedded crates. Spawns FFmpeg subprocesses for
  media work; spawns whisper.cpp for ASR.
- **Worker crates** (`crates/slop-*/`) - pure-Rust libraries that the host
  uses for individual jobs (probe, transcript, scenes, scoring, plan,
  validate, render).

There is no Python in the shipping binary. There is no separate web server.
The app runs offline by default; the only network call required is to the
user-configured LLM endpoint, which can itself be local (Ollama, llama.cpp).

## License posture

- Repo: MIT.
- FFmpeg: LGPL-2.1+ runtime; LGPL-only builds (no `--enable-gpl`).
- Default recommended local LLM: Qwen3 (Apache-2.0).
- Llama and Gemma are documented as alternatives with explicit license
  callouts; we do not bundle their weights.

See `docs/license-posture.md` for the full breakdown.

## Non-goals

See `docs/non-goals.md`. The short version: V1 is a rough-cut tool, not a
full NLE. Effects, transitions, color, multi-cam, and collaboration are
deliberately out of scope.
