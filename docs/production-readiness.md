# Production-readiness status

This document is the source of truth for what part of Slop AI is
production-ready and what isn't. Every row is one of:

- **green** — implemented, tested, verified by CI on `main`. Safe to ship.
- **yellow** — implemented and unit-tested locally, but not exercised
  end-to-end against real workloads in CI yet. Useful but not warranted.
- **red** — scaffolded only; the real implementation needs to be written
  and proven before it can be relied on. See [`docs/stubs.md`](stubs.md)
  for the punch list.

The green/yellow/red status is updated *only* after the corresponding CI
job passes on `main`. Do not promote rows based on local-only verification.

## Phase A snapshot (2026-05-06)

The current state after the first compile + test + lint + coverage pass.

### Workspace-level

| Item | Status | Acceptance criteria |
| --- | --- | --- |
| `cargo build --workspace` (default features) | green | `rust` job in CI green on macOS + Linux + Windows |
| `cargo test --workspace` (default features) | green | 140 tests pass, 0 failures |
| `cargo clippy --workspace -- -D warnings` | green | 0 warnings on default features |
| `cargo fmt --check` | green | exits 0 |
| `cargo deny check` | green | advisories + bans + licenses + sources all `ok` (with documented unmaintained ignores in `deny.toml`) |
| pnpm `-r typecheck` (strict) | green | 4 TS packages pass |
| `eslint --max-warnings 0` (desktop) | green | 0 warnings |
| Vitest suite (desktop store) | green | 6 tests pass |

### Per-crate (Rust)

| Crate | Compile | Unit tests | Real algorithm/impl status |
| --- | --- | --- | --- |
| slop-core | green | green (35 tests) | timeline + ops + reducer + validator + repair + v1->v2 migration: real, exhaustive |
| slop-media | yellow | yellow (3 tests) | ffprobe parsing real-tested; proxy/thumb/waveform shell out to ffmpeg, untested in CI without integration job |
| slop-asr | yellow | yellow (placeholder backend tested) | model download + diarization scaffolds; whisper.cpp/ort forwards are feature-gated stubs |
| slop-scenes | yellow | green (5 tests) | HSV-diff content detector real and tested; FrameStream decode requires ffmpeg in integration tests |
| slop-score | green | green (3 tests) | rule-based feature scoring real and unit-tested |
| slop-planner | yellow | green (compiles + structure tests) | OpenAI-compatible HTTP client real; quality is a function of the configured endpoint, no integration test |
| slop-otio | green | green (10 tests including XML escape and gap insertion) | pure-Rust OTIO subset writer + 3 adapter writers; all unit-tested |
| slop-render | yellow | green (4 tests) | filtergraph compilation real; cross-dissolve treated as a hard cut (see stubs.md) |
| slop-multicam | green | green (1 test) | FFT cross-correlation real; FFT-known-offset test passes within 0.01s |
| slop-color | green | green (8 tests) | ASC CDL math + .cube parser + trilinear interpolation: real and tested |
| slop-mixer | yellow | yellow (3 tests) | BS.1770-4 K-weighting math implemented; loudness coefficients use truncated f32 (tracked) |
| slop-transitions | green | green (5 tests) | xfade catalog + audio crossfade emitter: real |
| slop-captions | green | green (8 tests) | SRT + VTT + ASS writers with full styling |
| slop-vision | green | green (4 tests) | tile budget + frame extractor (frame extractor needs ffmpeg integration) |
| slop-agent | green | green (5 tests) | tools registry + JSON-Schema validation: real; agent loop network-dependent (deferred) |
| slop-genav | green | green (8 tests) | ComfyUI provider + XTTS provider + dub pipeline: skeletons + consent ledger real |
| slop-reframe | yellow | green (6 tests) | Kalman + crop solver real; YOLO inference path is feature-gated stub |
| slop-sync | yellow | green (6 tests) | Automerge doc + sync protocol round-trip works for 2-peer convergence; per-OpKind structured-field projection is partial (only `SetProjectSettings`) |
| slop-plugin | yellow | green (4 tests) | manifest + signature path real; component loading needs a real plugin to exercise |

### Apps

| App | Compile | Tests | Notes |
| --- | --- | --- | --- |
| apps/desktop (Tauri host) | green | n/a | IPC handlers wired for V1 commands. V1.5+/V2.0+/V2.5+/V3.0+ crates not yet exposed via IPC. |
| apps/sync-server | green | n/a | Compiles. End-to-end protocol test deferred. |
| apps/cli | green | n/a | Compiles. CLI smoke test against fixture project deferred. |

### Frontend

| Package | typecheck | lint | unit tests | Notes |
| --- | --- | --- | --- | --- |
| @slop/desktop | green | green | green (6 tests) | full strict typecheck, eslint -W 0, vitest store covered |
| @slop/web | green | n/a | n/a | typecheck only; PWA review companion has stub `replay()` |
| @slop/ui-timeline | green | n/a | n/a | typecheck only; component-level tests deferred to Phase B |
| @slop/schemas | n/a | n/a | n/a | JSON Schema validation passes; codegen produces 4 type files |

### Bindings

| Binding | Compile | Test | Notes |
| --- | --- | --- | --- |
| bindings/python (PyO3 0.24) | green | n/a | maturin build + import smoke deferred |
| bindings/node (napi-rs) | green | n/a | `napi build` + `require()` smoke deferred |

## Phase B priorities

Phase B converts yellow rows to green by landing real implementations and
their CI integration tests. See `docs/stubs.md` for the punch list.

The current Phase B priority order is the one in
[the original plan](../.cursor/plans/compile_and_test_the_workspace_75f0f4f6.plan.md):

1. Fill `slop-sync::doc::apply_op` for every `OpKind` variant.
2. Cross-dissolve render path + ffmpeg integration test.
3. CDL via baked 3D LUT replacing the `eq=brightness/gamma` approximation.
4. Tauri IPC handlers for V1.5 crates (multicam, color, mixer, transitions, captions).
5. Property-based tests (`proptest`) for slop-core.
6. Real ONNX pyannote pipeline + integration test.
7. Real whisper.cpp end-to-end + integration test.
8. Real YOLOv11 reframe + integration test.
9. `slop-cli` end-to-end test against a fixture project.
10. PyO3 + napi-rs build + integration tests.
11. ComfyUI / XTTS / OpenAI-compat translator integration tests.
12. `apps/web` real op replay (likely needs `slop-core` compiled to WASM).
13. Tauri IPC handlers for V2.0+/V2.5+/V3.0+ crates.
14. Wasmtime plugin sample + integration test.
15. Tauri build + signing/notarization on all three OSes.
16. Mutation + fuzz coverage targets.

Each item is one or more sessions, gated by CI, marked green only when the
corresponding CI job is green on `main`.
