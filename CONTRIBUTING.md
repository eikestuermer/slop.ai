# Contributing to Slop AI

Welcome. The full contributor guide lives in
[`docs/contributing.md`](docs/contributing.md). This file is a short pointer
so GitHub surfaces it next to the README.

## Required reading before your first PR

1. **[AGENTS.md](AGENTS.md)** — architectural pillars, workstream map, house
   rules. The invariants here are non-negotiable.
2. **[docs/onboarding.md](docs/onboarding.md)** — 10-minute walkthrough from
   `git clone` to a green CI run on your fork.
3. **[docs/contributing.md](docs/contributing.md)** — prerequisites, common
   tasks (`cargo test`, `pnpm typecheck`, `pnpm test`), PR conventions.
4. **[CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md)** — Contributor Covenant 2.1.
   Expected of everyone, AI agents included.

## Picking a task

- **First contribution?** [`docs/good-first-issues.md`](docs/good-first-issues.md)
  ranks the `S-*` punch-list entries by effort. Look for `S` (one-day) or `M`
  (one-week) items.
- **Full punch list** is [`docs/stubs.md`](docs/stubs.md). Every stub cites
  file:line, what's needed, the CI job that gates it, and an effort estimate.
- **Status board** is [`docs/production-readiness.md`](docs/production-readiness.md).
  When you turn a stub green, move its row from `docs/stubs.md` to "Closed"
  there in the same PR.
- **Larger architectural changes** go through the RFC process described in
  [`docs/governance.md`](docs/governance.md). Template:
  [`docs/rfcs/0000-template.md`](docs/rfcs/0000-template.md).

## How to claim work

Open an issue using the
[claim-stub template](.github/ISSUE_TEMPLATE/claim-stub.yml) with the `S-*`
id from `docs/stubs.md`. A maintainer will assign it; please don't open a PR
against an unassigned `S-*` without coordinating, so we don't waste your time.

## PR conventions (one screen)

- **Title format**: `<crate-or-area>: <imperative summary>`. Example:
  `slop-render: implement xfade for cross-dissolve (S-RENDER-001)`.
- **Reference the stub id** if your PR closes one. The CI integration job
  named in `docs/stubs.md` must go green.
- **Schema changes** must regenerate types in the same PR
  (`pnpm --filter @slop/schemas codegen`).
- **New runtime dependencies** must list license + advisory state in the
  description; `cargo deny check` must pass.
- **Architectural pillar checklist** in
  [`.github/PULL_REQUEST_TEMPLATE.md`](.github/PULL_REQUEST_TEMPLATE.md) must
  be filled in.

## Asking questions

- **Design / direction**: GitHub Discussions on this repo (see
  [DISCUSSIONS.md](DISCUSSIONS.md) for the policy and current status).
- **Bugs**: open an issue with the
  [bug template](.github/ISSUE_TEMPLATE/bug.yml).
- **Security**: do **not** open a public issue. See [SECURITY.md](SECURITY.md).

## Reviewing AI-generated PRs

This project explicitly welcomes AI contributors but holds them to the same
standards as humans. AI-authored PRs should still be small and reviewable,
must read `AGENTS.md` *before* coding, and must not silently introduce
features that contradict [`docs/non-goals.md`](docs/non-goals.md).
