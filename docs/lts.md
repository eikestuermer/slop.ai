# Long-Term Support (LTS) policy

Slop AI ships an LTS branch with every major release. Once a major
version (V1, V2, V3, ...) hits its `.5` polish release, the corresponding
branch becomes LTS for **2 years** from that date.

## Active LTS lines

| Version | LTS branch | Released | EOL |
| --- | --- | --- | --- |
| V3.5 | `release/v3` | (planned) | (planned) |
| V2.5 | `release/v2` | (planned, only if needed) | (planned) |

We keep at most two LTS lines active at a time (the current one and the
previous). Older lines move to "security-only" status on EOL date.

## What LTS gets

For 2 years after declaration:

- **Critical security fixes** — backported within 30 days.
- **High-severity bugs** — backported within 90 days.
- **Schema-compatible bug fixes** — backported on a best-effort basis.

LTS branches **do not** get:

- New features.
- Schema changes (the LTS branch's schema version is frozen).
- Major dependency upgrades except for security.

## Versioning

Slop AI follows [Semantic Versioning](https://semver.org/):

- Major (`X.0.0`) — breaking change to the canonical schema or to public
  Rust crate APIs. Requires a migration in
  [`crates/slop-core/src/migrations/`](crates/slop-core/src/migrations/).
- Minor (`X.Y.0`) — new features. Schema additions are allowed but must
  remain readable by older minors.
- Patch (`X.Y.Z`) — bug fixes only.

## How to consume LTS

For deployments that don't want to track main:

```bash
git clone https://github.com/slop-ai/slop.ai
cd slop.ai
git checkout release/v3   # the latest LTS branch
```

CI on the LTS branch is the same matrix as main but pinned at the LTS
toolchain version. Release artifacts on the LTS line are published to
GitHub Releases under tags like `v3.5.4-lts`.
