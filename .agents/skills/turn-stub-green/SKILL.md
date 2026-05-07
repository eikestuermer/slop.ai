---
name: turn-stub-green
description: >-
  Run the canonical Phase B iteration loop: pick a stub from docs/stubs.md,
  write the real implementation, add unit tests, replace the matching CI
  integration-test placeholder, push, watch CI, and only after CI proves
  the row green on main, move it from stubs.md to "Closed" and update
  docs/production-readiness.md from yellow/red to green. Use when the user
  asks to "implement S-XXX", "turn stub <id> green", or "Phase B session
  for <topic>". This is the canonical workflow for production-readiness
  iteration; do not skip steps.
---

# Turn a stub green (the Phase B iteration loop)

Phase B converts yellow / red rows in [docs/production-readiness.md](docs/production-readiness.md) into green rows by landing real implementations and the CI tests that prove them. Each session is one stub, one CI job promotion. The plan that defined this loop is at `~/.cursor/plans/compile_and_test_the_workspace_75f0f4f6.plan.md`.

## Inputs

- The stub id (e.g. `S-DOC-001`, `S-RENDER-001`, `S-DIAR-001`). Each id's row in [docs/stubs.md](docs/stubs.md) tells you the file:line, what's there now, what's needed, and which CI job will gate it.

## Workflow

### 1. Pre-read the stub

Open [docs/stubs.md](docs/stubs.md), find the row, read the entire entry. Particular attention to:

- **Location** — the file:line you'll touch.
- **Need** — the spec; do not improvise here.
- **CI gate** — the matching job in [.github/workflows/ci.yml](.github/workflows/ci.yml). This is what proves the row green.
- **Effort** — S/M/L/XL. If S/M, one session; if L, multiple sessions, finish a phase per session.

### 2. Write the real implementation

Replace the stub. Follow the rules in:

- [.cursor/rules/rust.mdc](.cursor/rules/rust.mdc) for Rust style + architectural pillars.
- [.cursor/rules/schemas.mdc](.cursor/rules/schemas.mdc) if you're touching JSON Schemas.
- [.cursor/rules/tauri-host.mdc](.cursor/rules/tauri-host.mdc) for IPC commands.
- [.cursor/rules/tauri-frontend.mdc](.cursor/rules/tauri-frontend.mdc) for TS / React.

The relevant subagent for the workstream knows the conventions in detail; if the stub touches the AI workstream, delegate the implementation work to the [ai-pipeline-specialist](.cursor/agents/ai-pipeline-specialist.md) subagent. Same for the other three workstreams.

### 3. Add unit tests

Mandatory: the change isn't done without tests. Local tests should cover:

- The happy path (the algorithm works as specified).
- One edge case (empty input, max-size input, adversarial input).
- Round-trip if the change involves serialization.

If the test needs ffmpeg / ONNX / a real model, mark it `#[ignore]` with a comment pointing at the integration job that exercises it instead. Don't `panic!`; return a clean `#[test]` failure.

### 4. Replace the CI integration-test placeholder

Open [.github/workflows/ci.yml](.github/workflows/ci.yml). Find the matching job; today its body is `echo "TODO: <stub-id> not yet implemented" ; exit 0`.

Replace with the real test body. The shape varies per integration; the [write-ci-integration-test](.agents/skills/write-ci-integration-test/SKILL.md) skill has reference templates per integration type.

The integration test must:
- Set up dependencies (download fixture media, install the right toolchain).
- Run the real binary against real inputs.
- Assert a quality metric — not just "exit 0". WER < threshold for ASR, DER > threshold for diarization, frame-checksum match for render, etc.

### 5. Run locally, push, watch

```bash
# Local sanity
cargo test --workspace --no-fail-fast
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all -- --check
pnpm -r typecheck

# Push
git add .
git commit -m "<workstream>: implement <stub-id> (<short summary>)"
git push

# Watch
gh run watch
```

### 6. Iterate on CI failures

The first CI run after a stub-implementation rarely passes. Common causes:

- macOS-only env diffs (you developed on Linux).
- Timeouts (the integration test is slower in CI).
- Missing apt-get install lines (system deps).
- Allowed-to-fail Tier 2 jobs that suddenly fail because we made them gating; revisit.

Iterate per session — don't let a half-green CI sit on main.

### 7. Mark the row green

Only after the matching CI job is **green on `main`** (not just on a PR branch):

1. Open [docs/production-readiness.md](docs/production-readiness.md) and update the row's status column from `yellow` / `red` to `green`. Cite the CI run number in the row.
2. Open [docs/stubs.md](docs/stubs.md) and **move** the entry from its workstream section to the `## Closed (was a stub, now real)` section at the bottom, with the PR + CI run links.
3. If the change adjusted any architectural document, update it. In particular: if you closed a row that was on the [docs/non-goals.md](docs/non-goals.md) list, edit non-goals.md too.

### 8. Journal

Run [journal-to-ai-wiki](.agents/skills/journal-to-ai-wiki/SKILL.md). The bronze layer was captured automatically by the stop hook; you write the silver-layer index entry pointing at it.

## What's not allowed

- **Marking a row green without the CI job being green on `main`.** This is the entire point of the strict CI gate. "Works on my machine" is not the standard.
- **Bundling unrelated changes into a stub PR.** One stub per PR. If you discover a bug while implementing `S-RENDER-001`, fix it in a separate PR (or document it as a new `S-` row in stubs.md and address it in a separate session).
- **Editing a different stub's CI job in the same PR.** Each stub's CI job belongs to its own promotion.

## Anti-patterns

- **"Done locally, will fix CI later"**. The plan explicitly says: no row goes green based on local-only verification. The CI gate is the standard.
- **Replacing `echo "TODO" ; exit 0` with `echo "implemented" ; exit 0`**. The job body must actually exercise the implementation against a real fixture. If you don't have a fixture yet, that's the first piece to land.
