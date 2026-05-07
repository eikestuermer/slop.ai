# integration-otio-roundtrip

Validates that every adapter in [crates/slop-otio/src/adapters/](../../../../crates/slop-otio/src/adapters/) emits a document that the matching upstream tool accepts.

## Job body

```yaml
- uses: actions/setup-python@v5
  with:
    python-version: "3.11"
- name: Install xmllint + otio-py
  run: |
    sudo apt-get update && sudo apt-get install -y libxml2-utils
    pip install opentimelineio
- name: Build slop CLI
  run: cargo build -p slop-cli --release
- name: Export every format
  run: |
    PROJ=examples/sample-projects/sample-interview
    ./target/release/slop export "$PROJ" --target otio    --out /tmp/cut.otio
    ./target/release/slop export "$PROJ" --target fcp7    --out /tmp/cut.xml
    ./target/release/slop export "$PROJ" --target fcpxml  --out /tmp/cut.fcpxml
    ./target/release/slop export "$PROJ" --target kdenlive --out /tmp/cut.kdenlive
- name: Validate OTIO via otioconvert
  run: otioconvert -i /tmp/cut.otio -o /tmp/cut.otioz
- name: Validate FCP7 XML
  run: xmllint --noout /tmp/cut.xml
- name: Validate FCPXML
  run: xmllint --noout /tmp/cut.fcpxml
- name: Validate Kdenlive MLT XML
  run: xmllint --noout /tmp/cut.kdenlive
```

## Notes

- `otioconvert` round-trips OTIO through every adapter Python OpenTimelineIO knows about; if our emitter is malformed, this fails immediately.
- `xmllint --noout` only catches well-formedness, not schema validity. For DTD/XSD validation, use the per-NLE schema if available.
- Add an FCP X Library round-trip via `fcpxml` once we have a CI runner with FCP installed (post-V3.0; tracked separately).

## Promotion criteria

- 3+ green runs.
- All four adapters validate.
- Then promote to gating; the row applies to all four `slop-otio` adapter rows in [docs/production-readiness.md](../../../../docs/production-readiness.md).
