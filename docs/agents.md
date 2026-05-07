# Agent setup for slop.ai

This is the contributor README for the workspace's Cursor agent infrastructure. Every file added by the [workspace_agent_setup](../../.cursor/plans/) plan is indexed here.

If you are an AI agent reading this for the first time, read [AGENTS.md](../AGENTS.md) first; it has the architectural pillars and the cross-cutting rules.

## TL;DR

- **5 rules** under [.cursor/rules/](../.cursor/rules/): glob-scoped guidance applied automatically when you edit matching files.
- **9 skills** under [.agents/skills/](../.agents/skills/): self-contained workflows you invoke by name or by description match.
- **4 subagents** under [.cursor/agents/](../.cursor/agents/): specialized assistants you delegate to via the Task tool when working on a specific workstream.
- **2 hooks** under [.cursor/hooks/](../.cursor/hooks/): an afterFileEdit advisory hook for schema edits and a beforeShellExecution gate for `cargo publish` / `cargo yank`.
- **1 marker file** [.ai-wiki-capture](../.ai-wiki-capture): opts the workspace into automatic AI-Wiki bronze capture via the user-global stop hook.

## Rules ([.cursor/rules/](../.cursor/rules/))

Rules are glob-scoped guidance that Cursor surfaces in your prompt context whenever you have matching files open or are editing them. They are short by design.

| File | Globs | What it covers |
| --- | --- | --- |
| [rust.mdc](../.cursor/rules/rust.mdc) | `**/*.rs`, `**/Cargo.toml` | Architectural pillars, Rust style, error handling, FFmpeg license posture, when to update [docs/stubs.md](stubs.md). |
| [tauri-host.mdc](../.cursor/rules/tauri-host.mdc) | `apps/desktop/src-tauri/**` | IPC command shape, V1.5+ IPC backlog, capability scope. |
| [tauri-frontend.mdc](../.cursor/rules/tauri-frontend.mdc) | `apps/desktop/src/**`, `apps/web/src/**`, `packages/ui-timeline/**` | Type sources, ESLint posture, i18n, Vitest patterns. |
| [schemas.mdc](../.cursor/rules/schemas.mdc) | `packages/schemas/**/*.json` | Schema-as-SoT discipline, codegen workflow, breaking changes require migrations. |
| [citation-standards.mdc](../.cursor/rules/citation-standards.mdc) | (no glob; description-triggered) | Inline `[Title](URL)` citations + mandatory Sources section for any web-research output. Copied verbatim from the Parallel plugin. |

## Skills ([.agents/skills/](../.agents/skills/))

Skills are reusable multi-step workflows. Cursor surfaces their existence in the agent prompt; the agent reads `SKILL.md` and follows it when the description matches the user request.

| Skill | Triggered by | What it does |
| --- | --- | --- |
| [add-rust-crate](../.agents/skills/add-rust-crate/SKILL.md) | "add a new crate", "scaffold slop-X" | Scaffold a new Rust crate that uses workspace deps, registers in root `Cargo.toml`, ships with a smoke test. |
| [add-op-variant](../.agents/skills/add-op-variant/SKILL.md) | "add an OpKind variant", "new reversible op" | End-to-end OpKind addition: schema + reducer + inverse + validator + repair + tests. |
| [add-otio-adapter](../.agents/skills/add-otio-adapter/SKILL.md) | "add export adapter for X", "support FCPX/AAF/EDL" | New module under `crates/slop-otio/src/adapters/` + escape-character round-trip tests. |
| [validate-schema-change](../.agents/skills/validate-schema-change/SKILL.md) | "I changed a schema", "regen types" | The canonical schema-edit workflow: validate, codegen, mirror Rust types, migration if breaking. |
| [turn-stub-green](../.agents/skills/turn-stub-green/SKILL.md) | "implement stub S-XXX", "Phase B session for X" | Master Phase B loop: pick stub from [stubs.md](stubs.md), real impl, CI integration, only mark green after CI proves it on `main`. |
| [write-ci-integration-test](../.agents/skills/write-ci-integration-test/SKILL.md) | "wire integration test for X" | Replace a Tier-2 placeholder body in [.github/workflows/ci.yml](../.github/workflows/ci.yml) with a real test. Has [references/](../.agents/skills/write-ci-integration-test/references/) per-integration templates (whisper, pyannote, yolo, ffmpeg-render, otio-roundtrip, sync, plugin, tauri, bindings-py, bindings-node, a11y). |
| [fix-cargo-deny-advisory](../.agents/skills/fix-cargo-deny-advisory/SKILL.md) | "RUSTSEC-XXXX", "cargo deny check failure" | Triage advisory, choose upgrade vs ignore-with-justification, document in [deny.toml](../deny.toml). |
| [regenerate-typescript-types](../.agents/skills/regenerate-typescript-types/SKILL.md) | "regen schemas", "TS types out of date" | Tiny helper for the codegen step. |
| [journal-to-ai-wiki](../.agents/skills/journal-to-ai-wiki/SKILL.md) | end-of-session for any meaningful task | Write the silver-layer index entry pointing at the (auto-captured) bronze transcript. |

## Subagents ([.cursor/agents/](../.cursor/agents/))

Subagents are specialized assistants with their own system prompts, invoked via the Task tool with `subagent_type` set to the subagent's name. They preserve context (the parent's conversation isn't bloated by their exploration) and they apply specialized expertise.

The four match the four 2-year roadmap workstreams.

| Subagent | Workstream | Owns |
| --- | --- | --- |
| [engine-architect](../.cursor/agents/engine-architect.md) | Engine | `slop-core`, `slop-render`, `slop-otio`, `slop-multicam`, `slop-color`, `slop-mixer`, `slop-transitions`, `slop-captions`, schemas, V2 migration, OpKind variants, validator/repair. |
| [ai-pipeline-specialist](../.cursor/agents/ai-pipeline-specialist.md) | AI | `slop-asr`, `slop-scenes`, `slop-score`, `slop-vision`, `slop-agent`, `slop-genav`, `slop-reframe`, `slop-planner`. ONNX, whisper.cpp, ComfyUI, XTTS-v2, pyannote. |
| [platform-collab-engineer](../.cursor/agents/platform-collab-engineer.md) | Platform | `slop-sync`, `slop-plugin`, `apps/sync-server`, `apps/desktop`, `apps/web`, `apps/cli`, `bindings/python`, `bindings/node`. |
| [community-curator](../.cursor/agents/community-curator.md) | Community | docs, governance, RFC process, license posture, security advisories, AI-Wiki, contributor onboarding. |

To delegate, the parent agent calls the Task tool with `subagent_type: "engine-architect"` (or another name) and a focused `prompt`. The subagent runs in an isolated context.

## Hooks ([.cursor/hooks/](../.cursor/hooks/))

Hooks fire on Cursor events and can advise (advisory message added to the next agent turn) or block (deny a tool action). Both ours are workspace-scoped at [.cursor/hooks.json](../.cursor/hooks.json).

| Hook | Event | What it does |
| --- | --- | --- |
| [schema-codegen-reminder.sh](../.cursor/hooks/schema-codegen-reminder.sh) | `afterFileEdit` (matcher: `Write|TabWrite`) | Filters script-side for edits to `packages/schemas/*.json` (excluding the gitignored `generated/` subtree) and emits a reminder to run `pnpm --filter @slop/schemas codegen` and update the matching Rust mirrors. Advisory only; never blocks. |
| [pre-shell-cargo-publish-gate.sh](../.cursor/hooks/pre-shell-cargo-publish-gate.sh) | `beforeShellExecution` (matcher: `cargo (publish|yank)`) | Blocks `cargo publish` and `cargo yank` unless an empty `.allow-cargo-publish` marker exists at the repo root. Publishing slop-* crates to crates.io is a governance decision; see [docs/governance.md](governance.md). `failClosed: true`. |
| [test-publish-gate.sh](../.cursor/hooks/test-publish-gate.sh) | (smoke test, not a hook) | Self-contained smoke test for the publish gate. Run via `./.cursor/hooks/test-publish-gate.sh`. |

## AI-Wiki capture ([.ai-wiki-capture](../.ai-wiki-capture))

Empty marker file. Its presence opts this workspace into AI-Wiki bronze capture: the user-global stop hook at `~/.cursor/hooks.json` calls `~/AI-Wiki/scripts/capture-session.sh` at the end of every agent turn, copying the session JSONL + a meta.yaml into `~/AI-Wiki/raw/transcripts/<date>-<uuid>/`.

Silver-layer entries are not automatic; you write them via [journal-to-ai-wiki](../.agents/skills/journal-to-ai-wiki/SKILL.md). The schema for silver entries is at `~/AI-Wiki/raw/agent-journal/AGENTS.md`.

To opt out for a single workspace: delete this file. The user-global stop hook fails open if the marker is missing.

## How to add to this setup

- **New rule**: drop a `.mdc` under [.cursor/rules/](../.cursor/rules/) following the `[create-rule](~/.cursor/skills-cursor/create-rule/SKILL.md)` spec. Glob-scope when possible. Keep ≤50 lines.
- **New skill**: create `.agents/skills/<kebab-name>/SKILL.md` following the `[create-skill](~/.cursor/skills-cursor/create-skill/SKILL.md)` spec. Use `references/` for progressive disclosure.
- **New subagent**: create `.cursor/agents/<kebab-name>.md` following the `[create-subagent](~/.cursor/skills-cursor/create-subagent/SKILL.md)` spec. The body is the system prompt verbatim.
- **New hook**: extend [.cursor/hooks.json](../.cursor/hooks.json) and add a script under [.cursor/hooks/](../.cursor/hooks/). Smoke-test with a local pipe; verify the matcher is JS regex (not POSIX) per the [create-hook](~/.cursor/skills-cursor/create-hook/SKILL.md) spec.

## Prior art

- The `.agents/skills/` location (rather than `.cursor/skills/`) follows lakehus precedent in `~/.cursor/plans/consolidate_skills_into_agents_2ffdbec1.plan.md`.
- The skills-as-workflows / rules-as-style framing follows `~/.cursor/plans/cursor_subagents_setup_13611cf9.plan.md`.
- The `references/` progressive-disclosure pattern is borrowed from the dbt plugin under `~/.cursor/plugins/cache/cursor-public/dbt/`.
- The citation-standards rule is a verbatim copy from the Parallel plugin under `~/.cursor/plugins/cache/cursor-public/parallel/`.
