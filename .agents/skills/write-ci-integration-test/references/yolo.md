# integration-yolo

Closes `S-YOLO-001` in [docs/stubs.md](../../../../docs/stubs.md).

## Fixture

- One JPEG at `crates/slop-reframe/tests/fixtures/yolo/person.jpg` containing a known person.
- Ground-truth bbox JSON at `crates/slop-reframe/tests/fixtures/yolo/person.bbox.json`: `{"cx": 0.5, "cy": 0.45, "w": 0.18, "h": 0.55}` (normalized).

## Job body

```yaml
- name: Cache YOLOv11 model
  id: model-cache
  uses: actions/cache@v4
  with:
    path: ~/.cache/slop/yolo/yolo11n.onnx
    key: yolo11n-onnx-v1
- name: Download yolo11n.onnx (~6 MB)
  if: steps.model-cache.outputs.cache-hit != 'true'
  run: |
    mkdir -p ~/.cache/slop/yolo
    curl -L -o ~/.cache/slop/yolo/yolo11n.onnx \
      https://github.com/ultralytics/assets/releases/download/v8.3.0/yolo11n.onnx
- name: Run YOLO integration suite
  run: cargo test -p slop-reframe --features ort --test yolo_e2e -- --nocapture
```

## Test (in `crates/slop-reframe/tests/yolo_e2e.rs`)

```rust
#[test]
#[cfg_attr(not(feature = "ort"), ignore)]
fn yolo_finds_person_at_expected_bbox() {
    let detector = YoloDetector::new(format!("{}/.cache/slop/yolo/yolo11n.onnx", std::env::var("HOME").unwrap()));
    let img = image::open("tests/fixtures/yolo/person.jpg").unwrap().to_rgb8();
    let (w, h) = (img.width(), img.height());
    let detections = detector.detect(img.as_raw(), w, h).unwrap();
    let truth: GroundTruth = serde_json::from_str(&std::fs::read_to_string("tests/fixtures/yolo/person.bbox.json").unwrap()).unwrap();
    let person = detections.iter().filter(|d| d.class_id == 0).max_by(|a, b| a.score.partial_cmp(&b.score).unwrap()).unwrap();
    assert!((person.cx - truth.cx).abs() < 10.0/w as f32, "cx off: {} vs {}", person.cx, truth.cx);
    assert!((person.cy - truth.cy).abs() < 10.0/h as f32, "cy off: {} vs {}", person.cy, truth.cy);
}
```

(Adds `image` to `[dev-dependencies]` in `crates/slop-reframe/Cargo.toml`.)

## Promotion criteria

- 3+ green runs.
- The 10px tolerance holds.
- Then promote to gating, mark `S-YOLO-001` green.
