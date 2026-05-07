# AGENTS.md — slop.ai

This file is the source of truth for AI contributors and humans working in
this repo. Read it before making changes. Conventions here override hunches.

A tour of every other piece of the agent setup lives at
[docs/agents.md](docs/agents.md).

## Architectural pillars (do not violate without an RFC)

These are stricter than style preferences. They are the design intent of the
project; violating them produces silent correctness bugs.

1. **Two-layer state.** The canonical project state is a `Timeline` JSON document conforming to [packages/schemas/timeline.v2.json](packages/schemas/timeline.v2.json), plus an append-only `ops.jsonl` command log. **OTIO is derived from this; it is never the source of truth.** If you find yourself reading OTIO to reconstruct project state, stop.
2. **Schema is single source of truth.** Rust types in [crates/slop-core/src/timeline.rs](crates/slop-core/src/timeline.rs) etc. mirror the schemas; TypeScript types in [packages/schemas/generated/](packages/schemas/generated/) are auto-generated. Never hand-edit the generated TS files. Schema changes follow the workflow in [.cursor/rules/schemas.mdc](.cursor/rules/schemas.mdc).
3. **Candidate-set planning.** The planner LLM only ever selects clips and moments from a precomputed candidate list. It never receives raw media URIs in the prompt. It never invents IDs. See [crates/slop-score/src/prompt_pack.rs](crates/slop-score/src/prompt_pack.rs).
4. **Validate then apply.** Every LLM JSON response goes through `slop_core::validator::validate_plan_semantics` before any timeline mutation. Invalid responses go to `slop_core::repair::repair_plan` and back through the validator. Never let raw model output mutate state.
5. **Deterministic render.** [crates/slop-render](crates/slop-render) compiles app-state to an FFmpeg filtergraph. The LLM never produces filtergraphs.
6. **Rough-cut promise only.** V1 supports cuts, trims, markers, captions, and a `speed` scalar. It does not support effect stacks, transitions beyond a simple cross-dissolve, or speed ramps. See [docs/non-goals.md](docs/non-goals.md).

## Where things live

### Crates

- [crates/slop-core](crates/slop-core) — canonical timeline schema (Rust types generated from JSON Schema), the `Op` enum (commands), the validator, the repair patcher, and the application reducer.
- [crates/slop-media](crates/slop-media) — FFmpeg / ffprobe wrappers (default render path).
- [crates/slop-asr](crates/slop-asr) — whisper.cpp + pyannote ONNX scaffolds (feature-gated under `whisper-cpp` and `ort`).
- [crates/slop-scenes](crates/slop-scenes) — port of PySceneDetect's `ContentDetector` and `AdaptiveDetector`.
- [crates/slop-score](crates/slop-score) — candidate moment builder + scorer.
- [crates/slop-planner](crates/slop-planner) — OpenAI-compatible chat-completions client.
- [crates/slop-otio](crates/slop-otio) — pure-Rust OTIO subset writer + Premiere FCP7 + Resolve FCPXML + Kdenlive MLT XML adapters.
- [crates/slop-render](crates/slop-render) — Timeline -> FFmpeg filtergraph compiler + runner.
- [crates/slop-multicam](crates/slop-multicam) — FFT-based audio cross-correlation for angle sync.
- [crates/slop-color](crates/slop-color) — ASC CDL math + Iridas `.cube` LUT loader + scope generators.
- [crates/slop-mixer](crates/slop-mixer) — ITU-R BS.1770-4 LUFS metering.
- [crates/slop-transitions](crates/slop-transitions) — `xfade` transition catalog.
- [crates/slop-captions](crates/slop-captions) — SRT / VTT / ASS writers.
- [crates/slop-vision](crates/slop-vision) — multi-modal frame tile extractor.
- [crates/slop-agent](crates/slop-agent) — agentic edit loop with OpenAI tool-call protocol.
- [crates/slop-genav](crates/slop-genav) — ComfyUI / XTTS-v2 / dubbing scaffolds + voice consent ledger.
- [crates/slop-reframe](crates/slop-reframe) — YOLO-driven smart-crop solver.
- [crates/slop-sync](crates/slop-sync) — Automerge-backed CRDT timeline + WebSocket sync protocol + ed25519 identity + ACL.
- [crates/slop-plugin](crates/slop-plugin) — WASI Component Model 0.2 plugin host on Wasmtime.

### Apps

- [apps/desktop](apps/desktop) — Tauri 2 desktop shell (React 18 + TS frontend, Rust IPC host).
- [apps/sync-server](apps/sync-server) — Axum + sled self-hosted sync server.
- [apps/cli](apps/cli) — `slop` headless CLI (ingest / plan / render / export).
- [apps/web](apps/web) — read-only PWA review companion.

### Bindings + packages

- [bindings/python](bindings/python) — PyO3 + maturin.
- [bindings/node](bindings/node) — napi-rs.
- [packages/schemas](packages/schemas) — JSON Schemas + TS codegen (single source of truth).
- [packages/ui-timeline](packages/ui-timeline) — shared React timeline canvas.

## Workstreams (the 2-year roadmap)

Phase B — see [docs/production-readiness.md](docs/production-readiness.md) and [docs/stubs.md](docs/stubs.md) — runs four parallel workstreams. Each maps to a Cursor subagent in [.cursor/agents/](.cursor/agents/).

| Workstream | Subagent | Owns |
| --- | --- | --- |
| Engine | [engine-architect](.cursor/agents/engine-architect.md) | `slop-core`, `slop-render`, `slop-otio`, `slop-multicam`, `slop-color`, `slop-mixer`, `slop-transitions`, `slop-captions`, schemas, V2 migration, OpKind variants, validator/repair |
| AI | [ai-pipeline-specialist](.cursor/agents/ai-pipeline-specialist.md) | `slop-asr`, `slop-scenes`, `slop-score`, `slop-vision`, `slop-agent`, `slop-genav`, `slop-reframe`, `slop-planner`. ONNX, whisper.cpp, ComfyUI, XTTS-v2, pyannote |
| Platform | [platform-collab-engineer](.cursor/agents/platform-collab-engineer.md) | `slop-sync`, `slop-plugin`, `apps/sync-server`, `apps/desktop`, `apps/web`, `apps/cli`, `bindings/python`, `bindings/node` |
| Community | [community-curator](.cursor/agents/community-curator.md) | docs, governance, RFC process, license posture, security advisories, AI-Wiki integration, contributor onboarding |

## Skills

Workspace skills live under [.agents/skills/](.agents/skills/). Each is a self-contained `SKILL.md` with progressive disclosure via `references/` where needed. The current set:

- [add-rust-crate](.agents/skills/add-rust-crate/SKILL.md) — scaffold a new `slop-*` crate.
- [add-op-variant](.agents/skills/add-op-variant/SKILL.md) — extend the `OpKind` enum end-to-end (schema → reducer → inverse → validator → repair → tests).
- [add-otio-adapter](.agents/skills/add-otio-adapter/SKILL.md) — add a new NLE export adapter.
- [validate-schema-change](.agents/skills/validate-schema-change/SKILL.md) — bump a schema version and write the migration.
- [turn-stub-green](.agents/skills/turn-stub-green/SKILL.md) — the canonical Phase B iteration loop.
- [write-ci-integration-test](.agents/skills/write-ci-integration-test/SKILL.md) — replace a Tier-2 placeholder in [.github/workflows/ci.yml](.github/workflows/ci.yml).
- [fix-cargo-deny-advisory](.agents/skills/fix-cargo-deny-advisory/SKILL.md) — handle a new RUSTSEC advisory or license rejection.
- [regenerate-typescript-types](.agents/skills/regenerate-typescript-types/SKILL.md) — run codegen after a schema change.
- [journal-to-ai-wiki](.agents/skills/journal-to-ai-wiki/SKILL.md) — write the silver-layer journal entry at end of session.

## Mistakes to avoid

- **Do not** make OTIO the canonical state. Always derive.
- **Do not** let the LLM emit filtergraphs, asset URIs, or any value that is not an ID from the candidate list.
- **Do not** silently swallow validator errors. Surface them to the user with the offending JSON path.
- **Do not** add features beyond rough-cut scope without updating [docs/non-goals.md](docs/non-goals.md) first.
- **Do not** skip the schema regeneration step when changing [packages/schemas/](packages/schemas/). The `frontend` CI job will catch you, but PR review costs more than running codegen locally.
- **Do not** mark a row green in [docs/production-readiness.md](docs/production-readiness.md) based on local-only verification. The gate is CI.
- **Do not** edit files under `packages/schemas/generated/`. They are gitignored and regenerated.
- **Do not** edit `Cargo.lock` to "fix" cargo-deny errors. Upgrade the dependency or add a documented ignore in [deny.toml](deny.toml).

## License posture (do not regress)

- Repo is MIT.
- FFmpeg is used as an LGPL-2.1+ runtime dependency. Builds must be LGPL only; do not enable `--enable-gpl`.
- Default recommended local LLM is Qwen3 (Apache-2.0). Llama and Gemma are documented as alternatives with explicit license callouts; we do not bundle their weights.
- Every new dependency added to [Cargo.toml](Cargo.toml) or any `package.json` should be reviewed for license compatibility and noted in the PR description.

See [docs/license-posture.md](docs/license-posture.md) for details.

## AI-Wiki journaling

This workspace opts in to AI-Wiki capture via the marker file [.ai-wiki-capture](.ai-wiki-capture). The user-global stop hook at `~/.cursor/hooks.json` will copy the session JSONL + meta.yaml into `~/AI-Wiki/raw/transcripts/<date>-<uuid>/` automatically.

For meaningful tasks, also write a silver journal entry in `~/AI-Wiki/raw/agent-journal/cursor-local/` per the schema in `~/AI-Wiki/raw/agent-journal/AGENTS.md`. The [journal-to-ai-wiki](.agents/skills/journal-to-ai-wiki/SKILL.md) skill formats this for you.

Required fields when bronze exists: `transcript`, `conversation-id`, `workspace`, `git-head-sha`, `git-branch`, `started-at`, `ended-at`. Required always: `model-used`, `task`, `tried`, `found`, `worked`, `didnt-work`, `outcome`, `mistakes-visible`. Use `none` / `unknown` / `not-exposed` honestly when a field doesn't apply.

## Sub-AGENTS.md

Crate- and package-level `AGENTS.md` files override this one for their scope. Always read the nearest one before editing inside a crate.
