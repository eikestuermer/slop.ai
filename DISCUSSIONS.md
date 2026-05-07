# Discussions

The project's preferred venue for design discussion, integration help,
"how should X work?" questions, and showing off what you built with
slop.ai is **GitHub Discussions** on this repo:

<https://github.com/slop-ai/slop.ai/discussions>

## Status

Discussions need to be enabled by a maintainer in the GitHub repo settings.
This file is the policy; the actual feature toggle lives at:

- Repo settings → "General" → "Features" → check "Discussions".
- Maintainers: see [`MAINTAINERS.md`](MAINTAINERS.md) for who can do this.

If the link above 404s, Discussions hasn't been enabled yet. In that case:

- **Bug?** Open an issue with the
  [bug template](.github/ISSUE_TEMPLATE/bug.yml).
- **Want to claim a stub?** Open an issue with the
  [claim-stub template](.github/ISSUE_TEMPLATE/claim-stub.yml).
- **Architectural proposal?** Open an issue with the
  [rfc-proposal template](.github/ISSUE_TEMPLATE/rfc-proposal.yml). If
  maintainers think it's worth a full RFC they'll ask for one in
  [`docs/rfcs/`](docs/rfcs/).
- **Anything else?** Open an issue with the
  [question template](.github/ISSUE_TEMPLATE/question.yml). When
  Discussions is enabled, we'll triage these into the right Discussion
  category and close the issue with a link.

## When Discussions is enabled

Maintainers should configure these categories at minimum:

| Category        | Purpose                                                            |
|-----------------|--------------------------------------------------------------------|
| Announcements   | Maintainer-only. Releases, governance, breaking changes.           |
| Q&A             | Anyone. "How do I do X?". Convertible to issues if it's actually a bug. |
| Ideas           | Anyone. Pre-RFC ideation. RFC-worthy items get promoted to `docs/rfcs/`. |
| Show and tell   | Anyone. Things people built with slop.ai.                          |
| General         | Anything that doesn't fit the above.                               |

## Why this two-stage policy

A young repo with Discussions disabled by default but a friendly fallback
in the issue tracker is the lowest-friction path: contributors don't hit a
404 wall, and we don't accumulate a sprawling Discussion graveyard before
there are people to read it.
