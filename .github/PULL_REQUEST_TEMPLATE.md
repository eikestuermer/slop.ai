<!--
Thanks for contributing. Please fill in every section. Empty sections are
treated as a request for changes.

Title format: `<crate-or-area>: <imperative summary>`. Reference the stub id
if your PR closes one. Example:
  slop-render: implement xfade for cross-dissolve (S-RENDER-001)
-->

## Summary

<!-- One paragraph: what does this PR do, and why? -->

## Stub / issue / RFC reference

- Closes: <!-- e.g. #123 (the claim-stub issue) -->
- Stub id from `docs/stubs.md`: <!-- e.g. S-RENDER-001, or "N/A" -->
- RFC: <!-- link to merged RFC, or "N/A" -->

## CI gate

<!--
Which CI job in `.github/workflows/ci.yml` proves this PR works? If you
added a new job, name it here. If a stub row required a new job and you
didn't add it, explain why.
-->

- CI job that newly goes green: <!-- e.g. integration-render-xfade -->

## Architectural pillar checklist

<!-- Tick each, or write "N/A — pillar not touched" with one line of why. -->

- [ ] **1. Two-layer state.** Any new state lives in app-state or in `ops.jsonl`,
      not in a side database, and is reversible.
- [ ] **2. Candidate-set planning.** No new path lets the LLM discover or
      reference media that wasn't in the precomputed candidate set.
- [ ] **3. Validate then apply.** Any new LLM output passes through the
      schema validator + repair pass before reaching the reducer.
- [ ] **4. Deterministic render.** Render output is a pure function of the
      validated app-state; no LLM output reaches the filtergraph compiler.
- [ ] **5. Rough-cut promise only.** If this adds a feature beyond the V1
      promise, `docs/non-goals.md` was updated *before* the code change.

## Schema / migration impact

<!-- Tick all that apply. -->

- [ ] No schema changes.
- [ ] Schema changes regenerated TypeScript types in this PR (`pnpm --filter @slop/schemas codegen`).
- [ ] New `Op` variant — added a migration in `crates/slop-core/src/migrations/`.
- [ ] Public IPC surface changed — frontend types regenerated, Vitest mocks updated.

## Test plan

<!--
What did you actually run, and what was the output? Reviewers will not
re-run this manually; the CI job is the source of truth, but please paste
local results here as well.
-->

- [ ] `cargo test --workspace` passes locally.
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` passes.
- [ ] `cargo deny check` passes (run if any dependency changed).
- [ ] `pnpm --filter @slop/schemas validate` passes (run if schemas changed).
- [ ] Frontend: `pnpm --filter @slop/desktop typecheck && pnpm --filter @slop/desktop test` passes (run if desktop changed).

## Follow-ups

<!--
Anything you intentionally left out. Open issues for each, or paste a list
of stub ids that this PR creates the seam for.
-->
