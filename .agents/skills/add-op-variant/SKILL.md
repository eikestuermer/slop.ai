---
name: add-op-variant
description: >-
  Add a new reversible OpKind variant to slop-core end-to-end: schema bump,
  enum entry, reducer apply + inverse, validator coverage, repair coverage,
  migration if breaking, and unit tests proving idempotence + replay.
  Use when the user asks to "add an OpKind variant", "support a new
  reversible op", or "extend the command log to include X". Cross-references
  S-DOC-001 in docs/stubs.md (the slop-sync structured-field projection
  stub) so callers know to handle the new variant in TimelineDoc::apply_op.
---

# Add an OpKind variant

The `OpKind` enum at [crates/slop-core/src/ops.rs](crates/slop-core/src/ops.rs) is the command set the reducer applies to a `Timeline`. Every variant has an inverse (computed by [reducer.rs](crates/slop-core/src/reducer.rs)::`apply`) so undo/redo and crash-recovery via op-log replay both work for free.

Adding a variant touches 7 files. Skip a step and the validator, repair pass, sync server, or replay test will catch it; better to do it in order.

## Workflow

### 1. Decide whether the variant is breaking

- **Non-breaking**: adds a new variant. Existing serialized op logs continue to deserialize correctly because `serde` accepts unknown-tag-rejection on read by default — but old code can't read NEW logs, which we're fine with because schema versions are linear.
- **Breaking**: removes / renames / changes the payload of an existing variant. This requires a schema version bump (`ops.v1.json` -> `ops.v2.json`) and a migration. **Don't do this lightly**; the V1 op log is part of the data format we ship.

### 2. Update the JSON Schema

Edit [packages/schemas/ops.v1.json](packages/schemas/ops.v1.json) (or `ops.v2.json` if you're bumping):

- Add the new tag to the `kind` enum.
- Document the `payload` shape so external tools can validate `ops.jsonl` files.

### 3. Update `OpKind` in [crates/slop-core/src/ops.rs](crates/slop-core/src/ops.rs)

Add the variant. Use `#[serde(rename = "...")]` only if the JSON tag must differ from the Rust identifier; we prefer 1:1 mapping.

### 4. Update the reducer in [crates/slop-core/src/reducer.rs](crates/slop-core/src/reducer.rs)

The `apply` function must:
- Mutate `Timeline` to reflect the op.
- **Compute and return the inverse op.** This is non-negotiable; undo, replay, and the sync server all rely on it. If the inverse is genuinely the same op (e.g. a toggle), document why with a comment.

### 5. Update [validator.rs](crates/slop-core/src/validator.rs)

If your variant has invariants that are not enforced by the schema (e.g. references to existing assets / tracks, ordering constraints), add them to `validate_timeline_semantics`. Add a unit test that constructs an invalid post-state and asserts the validator rejects it.

### 6. Update [repair.rs](crates/slop-core/src/repair.rs) if applicable

Repair is for fixing planner output, not human edits. If your new variant can be emitted by the planner via [crates/slop-planner](crates/slop-planner), add a repair rule that clamps invalid values; otherwise skip this step and document the reason.

### 7. Update [crates/slop-sync/src/doc.rs](crates/slop-sync/src/doc.rs)

The `TimelineDoc::apply_op` method must project the new variant into Automerge structured fields. Today only `SetProjectSettings` is projected (`S-DOC-001` in [docs/stubs.md](docs/stubs.md)); your new variant inherits the same gap. **Either:**
- Add the projection now (preferred — the more variants we project, the closer S-DOC-001 gets to closed), or
- Document the gap in stubs.md and rely on the audit-log replay path.

### 8. Tests

Add to `mod tests` in [reducer.rs](crates/slop-core/src/reducer.rs):

```rust
#[test]
fn my_op_apply_then_inverse_round_trips() {
    let mut tl = Timeline::empty();
    // ... bring tl into a known state ...
    let op = Op::new(OpKind::MyNew { /* ... */ });
    let inverse = apply(&mut tl, &op).unwrap();
    apply(&mut tl, &inverse).unwrap();
    // tl is now back to the known state
    assert_eq!(/* ... */);
}

#[test]
fn replay_includes_my_op() {
    // Apply MyNew through OpLog::push, save to disk, load, reconstruct, assert state.
}
```

These two tests are mandatory: round-trip + replay. If you can't write them, the inverse is wrong.

## Verify before handing off

- `cargo test -p slop-core` passes (existing 35+ tests still green, your new ones added).
- `cargo clippy -p slop-core -- -D warnings` passes.
- `pnpm --filter @slop/schemas validate` passes.
- The TS codegen at [packages/schemas/generated/ops.v1.ts](packages/schemas/generated/ops.v1.ts) reflects the new variant.

## Anti-patterns

- **Don't skip the inverse.** Every variant's inverse is checked at apply time; missing it makes undo silently wrong.
- **Don't add validator rules to the schema.** Schema rules are syntactic (types, ranges); semantic rules (referential integrity) live in `validator.rs`.
- **Don't `unwrap()` in `apply`.** Use the existing `Error` enum and propagate.
