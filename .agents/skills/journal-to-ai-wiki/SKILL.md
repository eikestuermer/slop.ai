---
name: journal-to-ai-wiki
description: >-
  Write a silver-layer journal entry to ~/AI-Wiki at the end of any
  meaningful slop.ai session. The bronze layer (full transcript + meta.yaml)
  is captured automatically by the user-global stop hook because slop.ai
  has the .ai-wiki-capture marker; this skill writes the structured silver
  index entry that makes the transcript findable. Use at the end of any
  session that landed real changes (not for trivial reads or single-file
  fixes). Required fields are tracked in
  ~/AI-Wiki/raw/agent-journal/AGENTS.md; honest "unknown" / "not-exposed"
  values are required when telemetry isn't available.
---

# Journal a slop.ai session to AI-Wiki

The user-global stop hook at `~/.cursor/hooks.json` automatically captures the bronze layer (verbatim session JSONL + meta.yaml) into `~/AI-Wiki/raw/transcripts/<date>-<uuid>/` because slop.ai has the [.ai-wiki-capture](.ai-wiki-capture) marker. Your job is the silver-layer index entry.

The full schema lives at `~/AI-Wiki/raw/agent-journal/AGENTS.md`. Read it before writing your first entry. The required fields when bronze exists are:

- `transcript:` — relative path to the bronze folder
- `conversation-id:` — the Cursor conversation UUID
- `workspace:` — `/Users/eike/slop.ai`
- `git-head-sha:` — `git rev-parse HEAD`
- `git-branch:` — `git rev-parse --abbrev-ref HEAD`
- `started-at:` / `ended-at:` — ISO8601

Always required: `model-used`, `task`, `tried`, `found`, `worked`, `didnt-work`, `outcome`, `mistakes-visible`. Use `none` / `unknown` / `not-exposed` honestly when a field doesn't apply or when telemetry isn't available; do not invent values.

## File location

`~/AI-Wiki/raw/agent-journal/cursor-local/<ISO8601>-<short-title>.md`

Filename convention: `YYYY-MM-DDThh-mm-ss-<short-kebab-title>.md`. One entry per file.

## Template

```markdown
# 2026-MM-DDTHH:MM:SS+00:00 - <short title>

- actor: cursor-local
- model-used: `Claude Opus 4.7`
- token-usage: not-exposed
- source-repo: /Users/eike/slop.ai
- workspace: /Users/eike/slop.ai
- conversation-id: <uuid>
- transcript: `raw/transcripts/<date>-<uuid>/`
- git-head-sha: <sha>
- git-branch: main
- started-at: 2026-MM-DDTHH:MM:SS+00:00
- ended-at: 2026-MM-DDTHH:MM:SS+00:00
- task: <one-line request>
- subagents-used: <list, or `none`>
- subagent-models: <or `none`>
- subagent-duration: <or `none`>
- subagent-call-count: <or `none`>
- subagents-did: <or `none`>
- subagents-output: <or `none`>
- skills-used: <list, or `none`>
- skills-impact: <one line per skill, or `none`>
- mistakes-visible: <bullet list — be honest about wrong turns, missed steps, places where the user had to correct course>
- tried: <bullet list>
- found: <bullet list>
- worked: <bullet list>
- didnt-work: <bullet list — be specific>
- outcome: <one line>
- touched: <links to wiki notes / sources>
- next: <or `none`>
```

## When to journal

Write a silver entry for:

- Any session that landed code changes.
- Any session that produced a plan, an architectural decision, or a stub-promotion.
- Any session that surfaced bugs the next agent should know about.

Do **not** write a silver entry for:

- One-line README edits.
- Read-only exploration.
- Aborted sessions where nothing was tried.

The bronze transcript is captured either way; silver is the retrieval layer.

## Anti-patterns

- **Don't write `mistakes-visible: none` when there were mistakes.** Wrong turns are valuable signal for future sessions.
- **Don't invent token usage / subagent durations.** If Cursor doesn't surface the value, write `not-exposed`.
- **Don't paste the full chat transcript into silver.** The transcript is in bronze; silver is the index.
- **Don't append to an existing silver entry.** One file per outcome; append-only at the directory level, not per-file.
