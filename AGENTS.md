# AGENTS.md - Slop AI

This file is the source of truth for AI contributors working in this repo.
Read it before making changes. Conventions here override hunches.

## Architecture invariants (do not violate without an ADR)

1. **Two-layer state.** The canonical project state is the JSON document
   defined by `packages/schemas/timeline.v1.json` plus the append-only command
   log `ops.jsonl`. OTIO is *derived* from that state; OTIO is never the
   source of truth. If you find yourself reading OTIO to reconstruct project
   state, stop.
2. **Schema is single-source-of-truth.** Rust and TypeScript types are both
   *generated* from `packages/schemas/timeline.v1.json` and
   `packages/schemas/ops.v1.json`. Do not hand-edit the generated files.
3. **Candidate-set planning.** The planner LLM only ever selects clips and
   moments from a precomputed candidate list. It does not get raw media URIs
   in the prompt. It does not invent IDs.
4. **Validate then apply.** Every LLM JSON response goes through
   `slop_core::validator` before any timeline mutation. Invalid responses go
   to `slop_core::repair` and back through the validator. Never let raw model
   output mutate state.
5. **Deterministic render.** `slop_render` compiles app-state to an FFmpeg
   filtergraph. The LLM never produces filtergraphs.
6. **Rough cuts only.** V1 supports cuts, trims, markers, captions, and a
   `speed` scalar. It does not support effect stacks, transitions beyond a
   simple cross dissolve, or speed ramps. See `docs/non-goals.md`.

## Where things live

- `crates/slop-core/` - timeline schema (Rust types generated from JSON Schema),
  the `Op` enum (commands), the validator, the repair patcher, and the
  application reducer.
- `crates/slop-media/` - FFmpeg/ffprobe wrappers. Shells out to `ffmpeg` and
  `ffprobe` by default. Optional `pyav-like` frame access not in V1.
- `crates/slop-asr/` - whisper.cpp integration. The default V1 ships a small
  pure-Rust placeholder when whisper.cpp is not built; `--features whisper-cpp`
  links the real backend.
- `crates/slop-scenes/` - port of PySceneDetect's `ContentDetector` (HSV diff
  threshold) and an `AdaptiveDetector`.
- `crates/slop-score/` - candidate moment builder. Rule-based first.
- `crates/slop-planner/` - HTTP client for any OpenAI-compatible chat-completions
  endpoint. Sends a JSON Schema, expects strict JSON back.
- `crates/slop-otio/` - writes the OTIO subset we need (Timeline, Stack, Track,
  Clip, ExternalReference, Marker, TimeRange) as JSON matching the OTIO
  schema. FCP7 XML and Resolve XML adapters live here too.
- `crates/slop-render/` - app-state -> FFmpeg filtergraph -> MP4.
- `apps/desktop/` - Tauri 2.x desktop shell. Frontend in `src/`, Rust host in
  `src-tauri/`.
- `packages/schemas/` - canonical JSON Schemas. Both Rust and TS regenerate
  from these.
- `packages/ui-timeline/` - reusable timeline canvas React components.
- `examples/sample-projects/` - small fixture projects for tests and demos.

## Coding conventions

- **Rust**: 2021 edition, `cargo fmt`, `cargo clippy --all-targets --all-features
  -- -D warnings`. Public APIs documented with rustdoc. Errors with `thiserror`
  in libraries, `anyhow` in binaries.
- **TypeScript**: strict mode on, ESLint + Prettier, no `any` outside generated
  files. Generated files live in `packages/schemas/generated/` and are
  regenerated, never edited by hand.
- **JSON Schema**: draft 2020-12. All schemas have a `$id` and a `version`
  field. Schema-breaking changes bump the major version (`timeline.v2.json`)
  and add a migration in `crates/slop-core/src/migrations/`.
- **Tests**: every crate has unit tests. End-to-end tests live in
  `crates/slop-core/tests/` and exercise full ingest -> plan -> render flows
  on `examples/sample-projects/`.

## License posture (do not regress)

- Repo is MIT.
- FFmpeg is used as an LGPL-2.1+ runtime dependency. Builds must be LGPL only;
  do not enable `--enable-gpl`.
- Default recommended local LLM is Qwen3 (Apache-2.0). Llama and Gemma are
  documented as alternatives with explicit license callouts; we do not bundle
  their weights.
- Every new dependency added to `Cargo.toml` or `package.json` should be
  reviewed for license compatibility and noted in the PR description.

## Mistakes to avoid

- **Do not** make OTIO the canonical state. Always derive.
- **Do not** let the LLM emit filtergraphs, asset URIs, or any value that is
  not an ID from the candidate list.
- **Do not** silently swallow validator errors. Surface them to the user with
  the offending JSON path.
- **Do not** add features beyond rough-cut scope without updating
  `docs/non-goals.md` first.
- **Do not** skip the schema regeneration step when changing
  `packages/schemas/`. The build will catch you, but PR review costs more.

## Sub-AGENTS.md

Crate- and package-level `AGENTS.md` files override this one for their scope.
Always read the nearest one before editing inside a crate or package.
