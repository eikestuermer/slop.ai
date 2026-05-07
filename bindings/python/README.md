# slop-ai (Python)

Python bindings for Slop AI's core. Provides a small, stable surface for
loading projects, inspecting timelines, exporting to OTIO, and rendering.

## Install

```bash
pip install slop-ai
```

Or build from source:

```bash
pip install maturin
maturin develop --release --manifest-path bindings/python/Cargo.toml
```

## Usage

```python
import slop_py

tl = slop_py.PyTimeline.load_ops("./my-project/ops.jsonl")
print(tl.duration_sec(), tl.n_tracks())
tl.export_otio("./my-project/cut.otio")
```

The full pipeline (ingest, transcribe, plan, render) is exposed via the
`slop` CLI. The Python bindings are intentionally narrow: they cover what
makes sense to drive from a notebook or a Python orchestrator.
