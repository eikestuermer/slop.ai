# integration-bindings-py

Closes `S-PY-001`. Builds the wheel, installs it, smoke-imports.

## Job body (already in `ci.yml`, repeated for reference)

```yaml
- uses: actions/setup-python@v5
  with:
    python-version: "3.11"
- run: pip install maturin
- name: Build + install + smoke
  working-directory: bindings/python
  run: |
    maturin build --release
    pip install --find-links target/wheels/ slop-ai
    python -c "import slop_py; t = slop_py.PyTimeline(); print(t.to_json())"
```

## Phase B extension

Add a richer smoke that exercises the export surface:

```python
import slop_py, json, tempfile, pathlib
t = slop_py.PyTimeline()
assert t.n_tracks() == 0
out = pathlib.Path(tempfile.mkdtemp()) / "empty.otio"
t.export_otio(str(out))
doc = json.loads(out.read_text())
assert doc["OTIO_SCHEMA"] == "Timeline.1"
```

## Promotion criteria

- Wheel builds on macOS + Linux.
- Smoke imports + exports work.
- Then promote, mark `S-PY-001` green.
