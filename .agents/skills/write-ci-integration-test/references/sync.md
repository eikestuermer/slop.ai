# integration-sync

Closes `S-DOC-001` (`slop-sync::doc::apply_op` projection) once every `OpKind` variant projects into Automerge structured fields.

## Test (in `crates/slop-sync/tests/concurrent.rs`)

```rust
#[test]
fn two_clients_with_concurrent_edits_converge() {
    use slop_sync::{TimelineDoc, SyncSession};
    use slop_core::{Op, OpKind, TrackKind};

    let mut alice = TimelineDoc::empty().unwrap();
    let mut bob = TimelineDoc::empty().unwrap();

    // Both apply the same initial op so they share a parent.
    let init = Op::new(OpKind::AddTrack {
        track_id: "v1".into(),
        kind: TrackKind::Video,
    });
    alice.apply_op(&init).unwrap();
    bob.apply_op(&init).unwrap();

    // Concurrent edits: alice adds an asset; bob adds a track.
    alice.apply_op(&Op::new(OpKind::AddAsset(/* ... */))).unwrap();
    bob.apply_op(&Op::new(OpKind::AddTrack { track_id: "a1t".into(), kind: TrackKind::Audio })).unwrap();

    // Two-peer sync until convergence.
    let mut a_session = SyncSession::new();
    let mut b_session = SyncSession::new();
    for _ in 0..6 {
        if let Some(msg) = a_session.generate_message(&mut alice).unwrap() {
            b_session.receive_message(&mut bob, &msg).unwrap();
        }
        if let Some(msg) = b_session.generate_message(&mut bob).unwrap() {
            a_session.receive_message(&mut alice, &msg).unwrap();
        }
    }

    // Reconstruct on both sides; assert identical.
    let alice_tl = alice.reconstruct().unwrap();
    let bob_tl = bob.reconstruct().unwrap();
    assert_eq!(alice_tl, bob_tl, "post-sync timelines must be identical");
    // And the joint state has both the asset and the audio track.
    assert_eq!(alice_tl.tracks.len(), 2);
    assert_eq!(alice_tl.assets.len(), 1);
}
```

## Job body

```yaml
- name: Run sync convergence integration
  run: cargo test -p slop-sync --test concurrent -- --nocapture
```

(No external services or models; this runs entirely in-process.)

## Why this gates `S-DOC-001`

If `apply_op` only projects `SetProjectSettings`, the test above fails: when alice replays bob's `AddAsset` op via the audit log, the structured-field projection is empty and `reconstruct()` ends up missing the asset depending on which side reconstructs first. The test forces a real projection.

## Promotion criteria

- 3+ green runs on `main`.
- Mark `S-DOC-001` green and move to "Closed" in stubs.md.
