# Onboarding: from `git clone` to first PR

Goal: in about ten minutes you should have a green local test suite, an
understanding of where the architectural lines are, and a stub picked from
[`docs/stubs.md`](stubs.md). Then it's about iteration speed, not ceremony.

> Heads-up: some integration paths (whisper.cpp, ONNX models, ComfyUI,
> XTTS-v2, Tauri code-signing) need extra tooling. Those are flagged as
> **maintainer-only** below — you do *not* need them to make a useful PR.

## 1. Required prerequisites (need these for a green local build)

| Tool      | Minimum  | How to install (macOS / Linux)                                  |
|-----------|----------|-----------------------------------------------------------------|
| Rust      | 1.75+    | `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \| sh` |
| Node      | 20+      | [nvm](https://github.com/nvm-sh/nvm) or your distro's package    |
| pnpm      | 9+       | `corepack enable && corepack prepare pnpm@latest --activate`     |
| FFmpeg    | 6+ LGPL  | `brew install ffmpeg` (macOS) / `apt install ffmpeg` (Debian)    |

**FFmpeg licensing**: do *not* install a `--enable-gpl` build for
development. The release artifact ships LGPL-only; building against
GPL-only filters at dev time is the fastest way to ship a license bug. See
[`docs/license-posture.md`](license-posture.md).

**Tauri prerequisites** (only needed if you plan to run `pnpm tauri dev`):
the canonical list is at <https://v2.tauri.app/start/prerequisites/>. On
macOS you'll need Xcode CLT; on Ubuntu you'll need `libwebkit2gtk-4.1-dev`
and friends.

## 2. Optional / maintainer-only

You don't need any of these to land a typical PR. They light up specific
integration paths:

| Tool             | Lights up                                | When you need it                                                         |
|------------------|------------------------------------------|--------------------------------------------------------------------------|
| whisper.cpp      | Real ASR (S-ASR-001)                     | If you're working on `crates/slop-asr/` and want to validate WER locally |
| Ollama / Qwen3   | Real planner / agent loop                | If you're working on `crates/slop-planner/` or `crates/slop-agent/`      |
| ONNX runtime     | pyannote diarization, YOLOv11 detection  | If you're working on `crates/slop-asr/diarize.rs` or `crates/slop-reframe/` |
| ComfyUI / XTTS-v2| Generative B-roll / voice cloning        | If you're working on `crates/slop-genav/`                                |
| Apple Dev cert   | macOS code-signing, notarization         | Maintainer-only (`S-TAURI-001`)                                          |
| AI-Wiki vault    | Long-form journaling per the user rule   | Maintainer-only — see [AGENTS.md](../AGENTS.md)                          |

## 3. Clone and bootstrap

```bash
git clone https://github.com/slop-ai/slop.ai
cd slop.ai

pnpm install                          # JS dev deps (schemas, frontend, scripts)
cargo fetch                           # populate the Rust dep cache
pnpm --filter @slop/schemas codegen   # generate TS types from JSON schemas
```

If `cargo fetch` errors on a host crate, it's almost always FFmpeg / libclang
missing. Re-read step 1.

## 4. Run the local gate

The same checks run in CI. Run them locally and they should be green:

```bash
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace

pnpm --filter @slop/schemas validate
pnpm --filter @slop/desktop typecheck
pnpm --filter @slop/desktop test
```

If `cargo test` finishes green you're set. There are 140+ tests across the
workspace and they finish in a couple of minutes on a modern laptop the
first time you run them, and well under a minute incrementally.

## 5. Run the desktop app (optional)

```bash
pnpm --filter @slop/desktop tauri dev
```

This requires the Tauri prerequisites from step 1. Drag a sample video from
[`examples/sample-projects/`](../examples/sample-projects/) onto the window;
the V1 ingest / scenes / candidates flow runs end-to-end without the LLM if
you don't have one configured.

## 6. Read the architectural lines

Before changing code, please skim these. They're short and the invariants
are non-negotiable:

1. [`AGENTS.md`](../AGENTS.md) — the contributor compass. Workstream map,
   skill index, where to find what.
2. [`docs/architecture.md`](architecture.md) — the long-form design.
3. [`docs/non-goals.md`](non-goals.md) — what V1 deliberately does *not* do.
   If your idea is on this list, the path forward is an RFC, not a PR.
4. [`docs/schema.md`](schema.md) — the JSON Schema design and migration
   policy.

## 7. Pick a task

Two paths:

**Path A — pick from the punch list.** Open
[`docs/good-first-issues.md`](good-first-issues.md) for stubs ranked by
effort. Open a [claim-stub issue](../.github/ISSUE_TEMPLATE/claim-stub.yml)
with the `S-*` id. A maintainer will assign it.

**Path B — propose something.** Open an
[RFC pre-discussion issue](../.github/ISSUE_TEMPLATE/rfc-proposal.yml). If
maintainers think it's worth a full RFC, they'll ask for one in
[`docs/rfcs/`](rfcs/) before code.

## 8. Open the PR

Follow [`.github/PULL_REQUEST_TEMPLATE.md`](../.github/PULL_REQUEST_TEMPLATE.md).
The architectural-pillar checklist is the most-skipped section; reviewers
will ask for it. The CI integration job named in `docs/stubs.md` must go
green for the PR to merge.

When the PR closes a stub, in the same PR:

1. Move the stub row from `docs/stubs.md` to "Closed (was a stub, now real)"
   with the PR link.
2. Flip the corresponding row in
   [`docs/production-readiness.md`](production-readiness.md) from amber to
   green.

That's it. Welcome aboard.
