# Release checklist

Steps to cut a release. Keep them strict; "we'll fix it in a patch" is how
projects get a reputation for instability.

## Pre-release

1. **Green CI on `main`.** All jobs must pass: rust matrix, schemas,
   frontend, license audit.
2. **Schema regen committed.** If any JSON Schema in `packages/schemas/`
   changed, run `pnpm --filter @slop/schemas codegen` and commit the
   generated TypeScript output.
3. **Manual smoke test** of the desktop app on all three platforms:
   - import a clip,
   - kick off proxies / transcript / scenes,
   - run the planner against a local Ollama,
   - render preview MP4,
   - export OTIO,
   - export FCP7 XML and import into Premiere (best-effort),
   - export FCPXML and import into Resolve (best-effort),
   - export Kdenlive XML and open in Kdenlive (best-effort).
4. **CHANGELOG.md updated.** Move items from `[Unreleased]` to the new
   version section. Date it.
5. **`docs/non-goals.md` reviewed.** If the release adds anything that was
   previously listed as a non-goal, the entry must be removed in the same
   commit.
6. **License audit clean.** `cargo deny check licenses` passes locally and
   in CI.

## Release

1. Bump versions in `Cargo.toml` (workspace), `apps/desktop/package.json`,
   `apps/desktop/src-tauri/tauri.conf.json`, and any other crate that
   diverged from the workspace version.
2. Commit "chore: release vX.Y.Z" on `main`.
3. Tag `vX.Y.Z` and push the tag.
4. The release workflow runs and produces draft GitHub releases for all
   three platforms.
5. Manually verify the artifacts open and run on at least one machine per
   platform.
6. Promote the draft release to published.

## Post-release

1. Open a `chore: bump to next dev version` PR that resets versions.
2. Add a fresh `[Unreleased]` section to CHANGELOG.md.
3. Announce in the project's preferred channels.

## What we deliberately do not do

- **No `--enable-gpl` FFmpeg builds.** Even if a feature is asking for it,
  no.
- **No bundling of model weights.** Models are downloaded by the user.
- **No telemetry on by default.** Opt-in only, and disabled in privacy mode.
- **No silent network calls.** Every outbound HTTP request is to the
  user-configured LLM endpoint.
