# integration-ffmpeg-render

Renders a known fixture project to MP4, asserts the output is decodable and a checksummed golden frame matches.

## Fixture

- A `slop-cli`-compatible project at `examples/sample-projects/sample-interview/` (already exists; small media files referenced by URI).
- A golden frame at `examples/sample-projects/sample-interview/golden/frame_00120.png` (frame 120 of the rendered output) with sha256 in `golden/SHA256SUMS`.

## Job body

```yaml
- name: Install ffmpeg
  run: sudo apt-get update && sudo apt-get install -y ffmpeg
- name: Build slop CLI
  run: cargo build -p slop-cli --release
- name: Render fixture project
  run: |
    ./target/release/slop render examples/sample-projects/sample-interview --out /tmp/cut.mp4
- name: Decode golden frame and compare
  run: |
    ffmpeg -y -i /tmp/cut.mp4 -vf "select=eq(n\,120)" -vsync vfr /tmp/frame.png
    cmp /tmp/frame.png examples/sample-projects/sample-interview/golden/frame_00120.png
    sha256sum /tmp/frame.png
- name: Verify against checksum
  run: cd examples/sample-projects/sample-interview/golden && sha256sum -c SHA256SUMS
```

## Notes

- The `cmp` is byte-exact; if encoder versions vary across runners, soften to "structural similarity > 0.99" via `ffmpeg -lavfi "ssim=stats_file=-"`.
- The fixture media (the actual MP4 inputs) should be tiny (1–2 MB total) to keep CI fast. Use synthetic generated content (`testsrc2`) if real footage isn't suitable.

## Promotion criteria

- 3+ green runs.
- Then promote to gating, mark the corresponding row in [docs/production-readiness.md](../../../../docs/production-readiness.md).
