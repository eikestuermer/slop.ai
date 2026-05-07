---
name: fix-cargo-deny-advisory
description: >-
  Handle a new RUSTSEC advisory or license rejection from cargo-deny.
  Use when the user mentions "RUSTSEC-XXXX-NNNN", "cargo deny check
  failure", "license rejected", or when the licenses CI job fails. Walks
  through reading the advisory, checking for an upstream fix, choosing
  upgrade vs ignore-with-justification, updating deny.toml with a documented
  reason, and recording the decision in docs/security-advisories.md when
  one exists. Aligns with the slop.ai license posture (MIT repo, LGPL-only
  FFmpeg, Apache-2.0 weights for default model).
---

# Fix a cargo-deny advisory

`cargo deny check` is a hard CI gate at [.github/workflows/ci.yml](.github/workflows/ci.yml)::licenses. When it fails, the failure is one of:

1. **Vulnerability** — a `RUSTSEC-YYYY-NNNN` advisory with a fix available upstream. Resolution: upgrade.
2. **Unmaintained** — a `RUSTSEC-YYYY-NNNN` advisory tagged "unmaintained". Resolution: usually ignore with documented reason; sometimes upgrade if a maintained successor exists.
3. **License rejected** — a transitive dep ships under a license not on the allow list. Resolution: add the license to the allow list (if compatible with MIT) or replace the dep.
4. **Wildcard / multi-version / unknown-source** — a structural issue in [Cargo.toml](Cargo.toml). Resolution: pin a version or add the workspace member.

## Workflow

### Vulnerability with fix available

1. Read the advisory: `https://rustsec.org/advisories/RUSTSEC-YYYY-NNNN`. Note the affected version range and the fixed version.
2. Try the upgrade: edit the relevant `Cargo.toml` to bump the dep, then `cargo update -p <crate> && cargo check --workspace`. Fix any API drift.
3. Re-run `cargo deny check` and `cargo test --workspace`.
4. Commit message: `deps: upgrade <crate> to <version> (RUSTSEC-YYYY-NNNN)`.

This is what Phase A did for `wasmtime 27 -> 36` and `pyo3 0.22 -> 0.24.1`.

### Vulnerability without a fix yet

1. Read the advisory; look at the upstream issue tracker for ETA.
2. Add an `ignore` entry in [deny.toml](deny.toml) under `[advisories].ignore`:
   ```toml
   { id = "RUSTSEC-YYYY-NNNN", reason = "<crate> via <transitive-path>; tracked upstream at <link>; revisit on <release-version-or-date>." }
   ```
3. The `reason` is non-optional in spirit. A `reason` of "ignore" is unacceptable; CI review should reject it.
4. Commit message: `deny: ignore RUSTSEC-YYYY-NNNN until upstream fix`.

### Unmaintained dep

Same as the no-fix case, but the `reason` should mention that no successor crate exists or that the maintained successor would force a major rewrite. Phase A did this for the GTK3 unmaintained advisories (Tauri Linux dep), `unic-*` (jsonschema 0.18 transitive), `proc-macro-error`, and `instant`.

### License rejected

1. Look up the license: is it OSI-approved? FSF-free? Compatible with MIT?
2. If yes (e.g. `CDLA-Permissive-2.0` which Phase A added), append it to `[licenses].allow` in [deny.toml](deny.toml).
3. If no (e.g. `GPL-3.0`), find a different dep. The Slop AI license posture is MIT-only at the repo level; we never link a GPL dep statically.
4. Commit message: `deny: allow <license-id> (<rationale>)`.

### Wildcard / multi-version

Phase A surfaced this when workspace `path = "..."` deps were treated as wildcards. The fix:

```toml
slop-foo = { path = "crates/slop-foo", version = "0.1" }
```

i.e. include the `version` field next to `path`. cargo-deny accepts it.

For multi-version warnings (the same crate appearing twice in the dep graph), Phase A set `multiple-versions = "allow"` because the duplicates are harmless and tracking them is expensive in this workspace size.

## Verify before handing off

- `cargo deny check` exits 0.
- `cargo test --workspace --no-fail-fast` still passes after any upgrade.
- The PR description references the advisory id and the rationale.
- If you added an ignore: the entry has a `reason` that names the dep, the upstream link, and a revisit milestone (date or upstream release).

## Anti-patterns

- **Don't blanket-ignore advisories.** Each one needs its own line and its own `reason`.
- **Don't add a license to the allow list without checking compatibility.** GPL family does not go on this list.
- **Don't downgrade to silence the gate.** If the only fix is "drop the dep", drop the dep — even if it means rewriting the feature.
- **Don't bypass the gate by removing `command: check` from the licenses CI job.** That's the gate; keep it.
