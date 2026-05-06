# Sample projects

These are tiny projects for tests, examples, and contributor onboarding.
None of them ship media in the repo; they reference public test footage by
URI and rely on the user to download it.

## `sample-interview/`

A 60-second interview snippet across two takes plus a single B-roll clip.
The provided `goal.txt` is what the planner is supposed to satisfy. The
expected golden output lives in `expected.plan.json`.

This project is the source of truth for:

- the validator: every test in `crates/slop-core/tests/` runs against it.
- the prompt regression suite: the planner's output is compared (loosely)
  to `expected.plan.json` to catch regressions.

## How to use a sample project

```bash
# Open in the desktop app
pnpm --filter @slop/desktop tauri dev -- --project examples/sample-projects/sample-interview

# Or run the planner directly via the CLI (planned in V1.1)
```

## Adding a new sample project

1. Create a folder `examples/sample-projects/<short-name>/`.
2. Add `README.md` describing the goal, the source media, and any prompt
   notes.
3. Add `goal.txt` with a single-line user prompt.
4. (Optional) add `expected.plan.json` if you want regression coverage.
5. Reference any external media via stable URIs (Internet Archive,
   Pexels, etc.) so the project is reproducible without bundling video.
