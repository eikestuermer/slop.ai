# Governance

Slop AI is a community-led project. Decisions are made transparently by a
small steering committee with input from contributors and users.

## Roles

- **Maintainer** — has commit + release rights. Maintainers are listed in
  [`MAINTAINERS.md`](../MAINTAINERS.md). New maintainers are nominated by
  existing maintainers and confirmed by a 2/3 majority of the steering
  committee.
- **Steering committee** — 3-7 maintainers who own architectural direction
  and dispute resolution. Membership rotates by community vote every 18
  months.
- **Contributor** — anyone who has had a PR merged. Contributors are
  listed in `CONTRIBUTORS.md` (auto-generated).
- **User** — anyone who runs Slop AI.

## How decisions get made

1. **Routine changes** (bug fixes, small features, doc updates) — direct
   PR by any contributor; one maintainer review.
2. **Substantive changes** (new crate, new schema field, new platform
   target, new dependency over 1 MB) — require an RFC. See below.
3. **Architectural / direction changes** (new release theme, breaking
   schema change, governance change) — require an RFC + 2/3 steering
   committee vote.

## RFC process

1. Open a PR adding a Markdown file under `docs/rfcs/NNNN-title.md`.
2. Use the template at `docs/rfcs/0000-template.md`.
3. The RFC stays open at minimum 7 days. Comments anywhere (PR, GitHub
   discussions, project chat) are welcome.
4. Steering committee resolves the RFC with one of: `accepted`,
   `rejected`, `revisit-after-X`.
5. Accepted RFCs are merged with a status header. Implementation PRs
   reference the RFC number.

## Code of conduct

We follow the [Contributor Covenant 2.1](https://www.contributor-covenant.org/version/2/1/code_of_conduct/).
Reports go to `conduct@slop.ai` and are handled by the steering committee.

## Security disclosure

See [`SECURITY.md`](../SECURITY.md). 90-day coordinated disclosure with a
private channel on `security@slop.ai` (also reachable as a GitHub Security
Advisory).

## Forking

Slop AI is MIT. You can fork, rename, and ship a derivative. We ask only
that you do not reuse the "Slop AI" trademark and the orange "S" mark on
your build's first-launch screen so users don't confuse forks with the
upstream.

## License changes

The repo license cannot be changed without:

- written agreement from every maintainer with substantive code in the
  affected files;
- a public RFC and 2-week comment period;
- a clean replacement license that the steering committee certifies as
  OSI-approved and FSF-approved.

The intent: make sustaining-mode fee-for-license relicensing structurally
impossible.
