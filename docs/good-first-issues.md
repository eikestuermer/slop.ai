# Good first issues

If you're new to slop.ai, pick one of these. They're the `S-*` stub rows
ranked by effort. The full punch list, with file:line, what's needed, and
the CI job that gates each, is in [`stubs.md`](stubs.md).

Before claiming, please read [`onboarding.md`](onboarding.md) (10 minutes)
and [`AGENTS.md`](../AGENTS.md). To claim, open an issue with the
[claim-stub template](../.github/ISSUE_TEMPLATE/claim-stub.yml).

## Effort: S (one day)

These are scoped to a single afternoon for someone familiar with the area.

| Stub id          | Workstream | Title                                                  |
|------------------|------------|--------------------------------------------------------|
| **S-COLOR-001**  | Engine     | Bake CDL via synthetic 3D LUT instead of `eq` filter   |
| **S-CLI-001**    | Platform   | End-to-end CI smoke test for the `slop` CLI            |
| **S-PY-001**     | Platform   | Build + import-test the PyO3 wheel in CI               |
| **S-NODE-001**   | Platform   | Build + require-test the napi-rs binary in CI          |

These are great first PRs: the seam exists, the CI job is named, and the
"production-ready" delta is small.

## Effort: M (about a week)

| Stub id          | Workstream    | Title                                                       |
|------------------|---------------|-------------------------------------------------------------|
| **S-DOC-001**    | Engine        | Project all `OpKind`s into Automerge for real CRDT merging  |
| **S-RENDER-001** | Engine        | Emit overlapping xfade for cross-dissolve at render time    |
| **S-WEB-001**    | Engine        | Implement web-companion timeline replay                     |
| **S-ASR-001**    | AI            | whisper.cpp integration test against ground-truth fixture   |
| **S-YOLO-001**   | AI            | Real YOLOv11 ONNX forward pass + NMS for reframe detector   |
| **S-GENAV-001**  | AI            | docker-compose CI integration for ComfyUI B-roll provider   |
| **S-VOICE-001**  | AI            | docker-compose CI integration for XTTS-v2 voice provider    |
| **S-PLUGIN-001** | Platform      | Example WASI Component plugin + host integration test       |
| **S-IPC-001**    | Platform      | One IPC tranche per release theme (multicam / color / ...)  |
| **S-VERIF-001**  | Verification  | `proptest` strategies for `Timeline` / `Op` / `Plan`        |
| **S-VERIF-002**  | Verification  | `cargo-mutants` weekly job, target ≥ 80% kill rate          |
| **S-VERIF-003**  | Verification  | `cargo-fuzz` targets for validator / OTIO / adapters / cube |
| **S-VERIF-004**  | Verification  | Real Playwright + axe-core CI run against web companion     |

## Effort: L (about a month)

| Stub id          | Workstream | Title                                                          |
|------------------|------------|----------------------------------------------------------------|
| **S-DIAR-001**   | AI         | Real pyannote 3.0 ONNX inference (segmentation + embedding)    |
| **S-TAURI-001**  | Platform   | Full `tauri build` + sign + notarize on macOS / Linux / Windows|

These are still very welcome contributions — they're just bigger and the
review will involve more back-and-forth on the design before code lands.

## Effort: XL

None currently. The XL-shaped work in the 2-year roadmap is captured per
release in [`CHANGELOG.md`](../CHANGELOG.md) (Unreleased → post-V1 SOTA
scaffolds), not as individual stubs.

---

## Tips for picking

- **Engine + effort=S** (`S-COLOR-001`) is the highest-leverage starter for
  someone who likes graphics / video pipelines.
- **Platform + effort=S** (`S-CLI-001`, `S-PY-001`, `S-NODE-001`) is the
  highest-leverage starter for someone who likes CI/CD plumbing.
- **Verification** (`S-VERIF-*`) is the highest-leverage starter for someone
  who's enthusiastic about testing rigour. Mutation testing on a young
  Rust codebase tends to find a lot.
- **AI** stubs require the most domain context (ONNX shapes, ASR fixtures,
  WER, DER); not impossible for a first PR, but expect more reviewer
  guidance.

If none of these feel right, that's also fine — open a
[question issue](../.github/ISSUE_TEMPLATE/question.yml) and we'll help
shape something for you.
