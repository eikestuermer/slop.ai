---
name: add-otio-adapter
description: >-
  Add a new NLE export adapter to slop-otio (e.g. Avid AAF, EDL CMX3600,
  Final Cut Pro Library). Use when the user asks to "add an export
  adapter", "support FCPX/AAF/EDL", or "round-trip to <NLE>". Produces a
  new module under crates/slop-otio/src/adapters/, the writer wired into
  lib.rs::adapters, escape-character round-trip tests, and a golden-file
  fixture under crates/slop-otio/tests/.
---

# Add an OTIO adapter to slop-otio

slop-otio's job is one-way export. The OTIO subset is canonical; each
adapter (`fcp7`, `fcpxml`, `kdenlive`) is a writer that transforms the
canonical `slop_core::Timeline` into the target NLE's XML/JSON format.

## What's already there

- [crates/slop-otio/src/lib.rs](crates/slop-otio/src/lib.rs) — top-level exports.
- [adapters.rs](crates/slop-otio/src/adapters.rs) — the `mod adapters;` re-exports.
- [adapters/fcp7.rs](crates/slop-otio/src/adapters/fcp7.rs) — Premiere FCP7 XML.
- [adapters/fcpxml.rs](crates/slop-otio/src/adapters/fcpxml.rs) — DaVinci Resolve / FCPX.
- [adapters/kdenlive.rs](crates/slop-otio/src/adapters/kdenlive.rs) — Kdenlive MLT XML.
- [tests/golden.rs](crates/slop-otio/tests/golden.rs) — golden-file regression suite.

## Workflow

### 1. New module under `adapters/`

```rust
// crates/slop-otio/src/adapters/<your-target>.rs

//! <Target NLE> export adapter.
//!
//! See https://<spec-url>/. Notes on what subset of the spec we emit and
//! which V1 features deliberately don't round-trip (transitions / effects /
//! speed ramps; rough-cut promise only).

use slop_core::{Timeline, TrackItem, TrackKind};
use std::path::Path;

/// Write a <target>-compatible document for `tl` to `out`.
pub fn write_<your_target>(tl: &Timeline, out: &Path) -> std::io::Result<()> {
    // ... build the XML/JSON string ...
    if let Some(p) = out.parent() {
        if !p.as_os_str().is_empty() {
            std::fs::create_dir_all(p)?;
        }
    }
    std::fs::write(out, doc)
}

fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}
```

### 2. Wire into [adapters.rs](crates/slop-otio/src/adapters.rs)

```rust
pub mod <your_target>;
pub use <your_target>::write_<your_target>;
```

And re-export at the crate root in [lib.rs](crates/slop-otio/src/lib.rs):

```rust
pub use adapters::{write_fcp7_xml, write_kdenlive_xml, write_resolve_fcpxml, write_<your_target>};
```

### 3. Add round-trip + escape tests in [tests/golden.rs](crates/slop-otio/tests/golden.rs)

Mandatory test set:

- **`<your_target>_contains_expected_clips_and_files`** — fixture timeline with 2 clips, 2 assets, write document, read back, assert clip count + URI presence.
- **`<your_target>_xml_escapes_special_characters`** — fixture timeline where one asset's URI contains `&`, one clip's `selection_reason` contains `<` `>` `"`. Write, assert the special characters appear in their escaped form and the raw forms do not.

### 4. Update [docs/export-fidelity.md](docs/export-fidelity.md)

Add a row to the "What survives the round trip" table for the new target. Be honest:

- Cuts: yes
- Markers: yes
- Captions: depends on target
- Speed scalar: depends on target
- Transitions / effects: explicitly no

### 5. (Optional) CI integration test

If the target NLE has a CLI validator (xmllint, otioconvert, the NLE itself in a CI container), add an `integration-otio-roundtrip` job step that runs it on the fixture document. This is a Phase B follow-up tracked under `S-OTIO-001` style entries in [docs/stubs.md](docs/stubs.md).

## Verify before handing off

- `cargo test -p slop-otio` — your two new tests pass plus the existing 12.
- `cargo clippy -p slop-otio -- -D warnings`.
- The XML you emit is well-formed: you can pipe it through `xmllint --noout`.
- The export-fidelity doc is updated.

## Anti-patterns

- **Don't promise effect / transition fidelity.** It's a stated non-goal in [docs/non-goals.md](docs/non-goals.md). Be explicit in the docstring about what does not round-trip.
- **Don't read OTIO** to drive the adapter. The canonical state is `slop_core::Timeline`; OTIO is a sibling target, not a source.
- **Don't skip the escape test.** XML in NLE files routinely contains apostrophes (`Q&A: "strong"`); a buggy escape function silently corrupts user data.
