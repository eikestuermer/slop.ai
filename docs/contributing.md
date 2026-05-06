# Contributing

Thanks for working on Slop AI. Read [`AGENTS.md`](../AGENTS.md) before making
changes; the architectural invariants there are non-negotiable.

## Prerequisites

- **Rust** 1.75+ (`rustup default stable`)
- **Node** 20+ and **pnpm** 9+
- **FFmpeg** 6+ on `PATH` (LGPL build only; do not install a `--enable-gpl`
  build for development)
- **Tauri prerequisites** for your platform: see
  <https://v2.tauri.app/start/prerequisites/>

Optional:

- **whisper.cpp** built locally if you want to test the real ASR backend
  (otherwise the placeholder backend produces synthetic transcripts).
- **Ollama** with `qwen3:8b` pulled, or any OpenAI-compatible server, to
  exercise the planner.

## First-time setup

```bash
git clone https://github.com/slop-ai/slop.ai
cd slop.ai
pnpm install
cargo fetch
pnpm --filter @slop/schemas codegen
```

## Common tasks

```bash
# Build all Rust crates and run unit tests
cargo test

# Lint Rust
cargo clippy --all-targets --all-features -- -D warnings
cargo fmt --check

# Validate JSON Schemas
pnpm --filter @slop/schemas validate

# Regenerate TypeScript types after a schema change
pnpm --filter @slop/schemas codegen

# Run the desktop shell in dev mode
pnpm --filter @slop/desktop tauri dev
```

## PR conventions

- Title format: `<crate-or-area>: <imperative summary>`. Examples:
  - `slop-core: support compound clips in the reducer`
  - `apps/desktop: add waveform peaks to the timeline canvas`
  - `docs: clarify candidate-set planning`
- Every PR that changes a JSON Schema must regenerate types in the same PR.
- Every PR that adds a runtime dependency must note the license in the
  description.
- CI must pass: `cargo test`, `cargo clippy -D warnings`, `pnpm typecheck`,
  `pnpm test`, schema validation, and license check.

## Reviewing AI-generated PRs

This project explicitly welcomes AI contributors but holds them to the same
standards as humans:

- AI-authored PRs should still be small and reviewable.
- Read `AGENTS.md` *before* coding, not as a sanity check after.
- If the change touches anything in `docs/non-goals.md`, that file gets
  updated first.

## Code of conduct

Be kind, be honest about what doesn't work, and surface tradeoffs explicitly
rather than smoothing them over. The project's review culture is more
"engineering critique" than "lgtm party".
