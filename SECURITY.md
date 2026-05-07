# Security policy

## Reporting a vulnerability

Email `security@slop.ai` or open a GitHub Security Advisory on
[github.com/slop-ai/slop.ai](https://github.com/slop-ai/slop.ai). Both
channels are monitored by the steering committee.

Please include:

- the affected version(s) (commit SHA if main),
- a minimal reproduction,
- the security impact (data exposure, code execution, etc.).

Do **not** open a public issue or PR until we agree on a coordinated
disclosure date.

## Our commitments

- We acknowledge reports within **3 business days**.
- We aim to ship a fix within **30 days** for high/critical issues; lower
  severity issues are batched into the next release.
- We coordinate disclosure with reporters, and credit them in
  [`CHANGELOG.md`](CHANGELOG.md) unless they prefer otherwise.
- Embargo period defaults to **90 days**.

## Scope

In scope: the desktop app, the sync server, every published crate, the
CLI, the web companion, the bindings packages, and the official Docker
images.

Out of scope: third-party plugins (each plugin's repo handles its own
security disclosures), user-configured LLM endpoints (those are the
operator's responsibility), and forks of Slop AI not maintained by this
project.

## Hardening checklist

For maintainers reviewing security-sensitive PRs:

- New dependency? Run `cargo deny check`.
- New network endpoint? Default to localhost-only and surface a warning if
  the user binds to `0.0.0.0`.
- New filesystem write? Bind to the project root via the Tauri capability
  scope; never write outside.
- New plugin capability? Update [`docs/governance.md`](docs/governance.md)
  and require a new ABI version.
- Any change to the consent ledger format? RFC required.
